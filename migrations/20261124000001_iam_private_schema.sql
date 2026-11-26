-- =====================================================================
-- 1 of 3: private schema (IAM v1, minimal)
--
--   iam   private tables, written ONLY by the IAM admin service (role iam_writer)
--
-- Followed by:
--   2: iam_api_v1 public read contract (role iam_reader)
--   3: change notifications (triggers on the tables below)
--
-- Apply as a dedicated owner role (not a superuser, not the writer). Postgres 13+.
-- Creating the group role needs CREATEROLE once; otherwise create it beforehand.
-- Ops step, outside these migrations (one login role per service):
--   CREATE ROLE iam_admin_svc LOGIN PASSWORD '...' IN ROLE iam_writer;
--
-- Rules
--   * One kind of grant: (user, node, role). It covers the node's whole subtree, never
--     upward. Additive only (no deny): effective access = union of grants.
--   * Fine-grained access uses the tree: put a grower's stations under a leaf node and
--     grant the grower on that leaf. A device belongs to exactly one node.
--   * Tags describe, they never authorise.
--   * nodes.path (ltree) is maintained by trigger / move_node(); never set it by hand.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS ltree;   -- installs into `public`

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'iam_writer') THEN
    CREATE ROLE iam_writer NOLOGIN;
  END IF;
END $$;

CREATE SCHEMA iam;

-- ---------------------------------------------------------------------
-- Private tables
-- ---------------------------------------------------------------------

CREATE TABLE iam.nodes (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  parent_id       uuid REFERENCES iam.nodes(id),     -- NULL = a root
  path            ltree NOT NULL UNIQUE,             -- built from ids, never names
  name            text NOT NULL,
  tags            text[] NOT NULL DEFAULT '{}',
  keycloak_org_id text UNIQUE,                       -- non-null = tenant (org) boundary
  created_at      timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT nodes_max_depth CHECK (nlevel(path) <= 8)
);
CREATE INDEX nodes_path_gist  ON iam.nodes USING gist (path);
CREATE INDEX nodes_parent_idx ON iam.nodes (parent_id);

CREATE TABLE iam.users (
  id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  keycloak_sub text NOT NULL UNIQUE,
  email        text NOT NULL,
  home_node_id uuid REFERENCES iam.nodes(id),
  status       text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
  created_at   timestamptz NOT NULL DEFAULT now()
);

-- Placement only. A device with no row here is reachable by nobody.
CREATE TABLE iam.devices (
  device_id  text PRIMARY KEY,
  node_id    uuid NOT NULL REFERENCES iam.nodes(id),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX devices_node_idx ON iam.devices (node_id);

CREATE TABLE iam.grants (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    uuid NOT NULL REFERENCES iam.users(id),
  node_id    uuid NOT NULL REFERENCES iam.nodes(id),
  role       text NOT NULL CHECK (role IN ('admin', 'manager', 'member')),
  granted_by uuid REFERENCES iam.users(id),          -- NULL = system / bootstrap
  granted_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz,
  revoked_at timestamptz                             -- soft revoke
);
-- one ACTIVE grant per (user, node, role); also serves lookups by user_id
CREATE UNIQUE INDEX grants_active_uq
  ON iam.grants (user_id, node_id, role) WHERE revoked_at IS NULL;

-- Append-only record of changes. The writer inserts one row in the SAME transaction as
-- each change (grant, revoke, node move, device assign, user status change, ...).
-- Immutability is enforced by privileges (the writer has INSERT and SELECT only) and by
-- the triggers below, which reject update, delete and truncate.
CREATE TABLE iam.audit_log (
  id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  at          timestamptz NOT NULL DEFAULT now(),
  actor_id    uuid REFERENCES iam.users(id),         -- NULL = system
  action      text NOT NULL,                         -- e.g. 'user.status', 'grant.revoke', 'node.move'
  target_type text NOT NULL,                         -- 'user' | 'node' | 'device' | 'grant'
  target_id   text NOT NULL,
  detail      jsonb NOT NULL DEFAULT '{}'            -- e.g. {"from":"active","to":"disabled"}
);
CREATE INDEX audit_target_idx ON iam.audit_log (target_type, target_id, at DESC);

CREATE FUNCTION iam.forbid_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'iam.% is append-only', TG_TABLE_NAME;
END $$;

CREATE TRIGGER audit_log_no_update
  BEFORE UPDATE OR DELETE ON iam.audit_log
  FOR EACH ROW EXECUTE FUNCTION iam.forbid_mutation();
CREATE TRIGGER audit_log_no_truncate
  BEFORE TRUNCATE ON iam.audit_log
  FOR EACH STATEMENT EXECUTE FUNCTION iam.forbid_mutation();

-- ---------------------------------------------------------------------
-- Tree maintenance
-- ---------------------------------------------------------------------

CREATE FUNCTION iam.nodes_set_path() RETURNS trigger
LANGUAGE plpgsql
SET search_path = iam, public, pg_temp AS $$
DECLARE
  parent_path ltree;
  lbl text := replace(NEW.id::text, '-', '_');
BEGIN
  IF NEW.parent_id IS NULL THEN
    NEW.path := text2ltree(lbl);
  ELSE
    SELECT path INTO parent_path FROM nodes WHERE id = NEW.parent_id;
    IF parent_path IS NULL THEN
      RAISE EXCEPTION 'parent node % not found', NEW.parent_id;
    END IF;
    NEW.path := parent_path || text2ltree(lbl);
  END IF;
  RETURN NEW;
END $$;

CREATE TRIGGER nodes_set_path_trg
  BEFORE INSERT ON iam.nodes
  FOR EACH ROW EXECUTE FUNCTION iam.nodes_set_path();

-- Reparent a node and its whole subtree. Call inside the writer's transaction.
CREATE FUNCTION iam.move_node(p_node uuid, p_new_parent uuid) RETURNS void
LANGUAGE plpgsql
SET search_path = iam, public, pg_temp AS $$
DECLARE
  old_path ltree;
  new_parent_path ltree;
BEGIN
  SELECT path INTO old_path        FROM nodes WHERE id = p_node       FOR UPDATE;
  SELECT path INTO new_parent_path FROM nodes WHERE id = p_new_parent FOR UPDATE;
  IF old_path IS NULL OR new_parent_path IS NULL THEN
    RAISE EXCEPTION 'node or new parent not found';
  END IF;
  IF new_parent_path <@ old_path THEN
    RAISE EXCEPTION 'cannot move a node into its own subtree';
  END IF;
  UPDATE nodes SET parent_id = p_new_parent WHERE id = p_node;
  UPDATE nodes
     SET path = new_parent_path || subpath(path, nlevel(old_path) - 1)
   WHERE path <@ old_path;
END $$;

-- ---------------------------------------------------------------------
-- Privileges: the writer gets the private tables only.
-- (New schemas grant nothing to PUBLIC, so no revokes are needed.)
-- ---------------------------------------------------------------------

GRANT USAGE ON SCHEMA iam TO iam_writer;
GRANT SELECT, INSERT, UPDATE, DELETE ON iam.nodes   TO iam_writer;
GRANT SELECT, INSERT, UPDATE         ON iam.users   TO iam_writer;   -- disable users, don't delete
GRANT SELECT, INSERT, UPDATE, DELETE ON iam.devices TO iam_writer;
GRANT SELECT, INSERT, UPDATE         ON iam.grants  TO iam_writer;   -- soft revoke, no delete
GRANT SELECT, INSERT                 ON iam.audit_log TO iam_writer; -- append-only
GRANT EXECUTE ON FUNCTION iam.move_node(uuid, uuid) TO iam_writer;
