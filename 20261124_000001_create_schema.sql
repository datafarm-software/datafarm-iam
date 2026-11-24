-- =====================================================================
-- 0001_iam_schema.sql  (IAM v1, minimal)
--
--   iam         private tables, written ONLY by the IAM admin service (role iam_writer)
--   iam_api_v1  the public read contract; downstream services (role iam_reader)
--               can use ONLY this. Breaking changes ship as iam_api_v2; additive
--               changes bump meta.minor.
--
-- Apply as a dedicated owner role (not a superuser, not the writer). Postgres 13+.
-- Creating the two group roles needs CREATEROLE once; otherwise create them beforehand.
-- Ops step, outside this migration (one login role per service):
--   CREATE ROLE iam_admin_svc LOGIN PASSWORD '...' IN ROLE iam_writer;
--   CREATE ROLE charting_svc  LOGIN PASSWORD '...' IN ROLE iam_reader;
--
-- Rules
--   * One kind of grant: (user, node, role). It covers the node's whole subtree, never
--     upward. Additive only (no deny): effective access = union of grants.
--   * Fine-grained access uses the tree: put a grower's stations under a leaf node and
--     grant the grower on that leaf. A device belongs to exactly one node.
--   * Tags describe, they never authorise.
--   * nodes.path (ltree) is maintained by trigger / move_node(); never set it by hand.
--   * Contract functions return nothing for disabled users and revoked/expired grants.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS ltree;   -- installs into `public`

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'iam_writer') THEN
    CREATE ROLE iam_writer NOLOGIN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'iam_reader') THEN
    CREATE ROLE iam_reader NOLOGIN;
  END IF;
END $$;

CREATE SCHEMA iam;
CREATE SCHEMA iam_api_v1;

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
  external_ref    text UNIQUE,                       -- e.g. 'knack:4412', for re-runnable imports
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
-- Change notifications (part of the contract)
-- Channel `iam_v1_changes`, delivered at commit. Payload is IDs only, e.g.
--   {"entity":"grant","op":"U","id":"<grant>","user_id":"<user>","node_id":"<node>"}
--   entity: node | user | device | grant        op: I | U | D
-- Consumers invalidate caches: grant and user events by user_id; node and device events
-- flush everything (placement changed, so any user's reachable devices may have changed).
-- Changing the payload shape means a new channel name, like a new schema version.
-- ---------------------------------------------------------------------

CREATE FUNCTION iam.notify_change() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  r record;
  j jsonb;
BEGIN
  IF TG_OP = 'DELETE' THEN r := OLD; ELSE r := NEW; END IF;
  j := to_jsonb(r);
  PERFORM pg_notify('iam_v1_changes', jsonb_strip_nulls(jsonb_build_object(
    'entity',    TG_ARGV[0],
    'op',        left(TG_OP, 1),
    'id',        j -> 'id',
    'user_id',   j -> 'user_id',
    'node_id',   j -> 'node_id',
    'device_id', j -> 'device_id'
  ))::text);
  RETURN NULL;
END $$;

-- Nodes notify only on create, delete and parent changes (renames and tag edits don't
-- affect access), so moving a subtree produces one notification, not one per descendant.
CREATE TRIGGER nodes_notify
  AFTER INSERT OR DELETE OR UPDATE OF parent_id ON iam.nodes
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('node');
CREATE TRIGGER users_notify
  AFTER INSERT OR DELETE OR UPDATE OF status, home_node_id ON iam.users
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('user');
CREATE TRIGGER devices_notify
  AFTER INSERT OR DELETE OR UPDATE OF node_id ON iam.devices
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('device');
CREATE TRIGGER grants_notify
  AFTER INSERT OR UPDATE OR DELETE ON iam.grants
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('grant');

-- ---------------------------------------------------------------------
-- Public contract: iam_api_v1
-- Views run with the owner's rights; functions are SECURITY DEFINER with a pinned
-- search_path, so readers never need access to the iam schema.
-- ---------------------------------------------------------------------

CREATE VIEW iam_api_v1.meta AS
  SELECT 1 AS major, 0 AS minor;

-- Where every device sits. No personal data.
CREATE VIEW iam_api_v1.device_placement AS
  SELECT d.device_id, d.node_id, n.path::text AS node_path
    FROM iam.devices d
    JOIN iam.nodes n ON n.id = d.node_id;

-- Who is this token's subject? Zero rows = unknown user.
-- home_org_id = nearest ancestor-or-self org boundary of the home node.
CREATE FUNCTION iam_api_v1.user_context(p_sub text)
RETURNS TABLE (user_id uuid, status text, home_node_id uuid, home_org_id text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = iam, public, pg_temp AS $$
  SELECT u.id,
         u.status,
         u.home_node_id,
         (SELECT o.keycloak_org_id
            FROM nodes o
           WHERE o.path @> hn.path AND o.keycloak_org_id IS NOT NULL
           ORDER BY nlevel(o.path) DESC
           LIMIT 1)
    FROM users u
    LEFT JOIN nodes hn ON hn.id = u.home_node_id
   WHERE u.keycloak_sub = p_sub
$$;

-- The user's active grants with node paths: the compact scope a service can cache.
CREATE FUNCTION iam_api_v1.scope(p_user uuid)
RETURNS TABLE (grant_id uuid, node_id uuid, node_path text, role text, expires_at timestamptz)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = iam, public, pg_temp AS $$
  SELECT g.id, g.node_id, n.path::text, g.role, g.expires_at
    FROM grants g
    JOIN nodes n ON n.id = g.node_id
    JOIN users u ON u.id = g.user_id AND u.status = 'active'
   WHERE g.user_id = p_user
     AND g.revoked_at IS NULL
     AND (g.expires_at IS NULL OR g.expires_at > now())
   ORDER BY n.path
$$;

-- Roles the user holds on one device (grants on the device's node or any ancestor).
-- Zero rows = no access.
CREATE FUNCTION iam_api_v1.roles_on_device(p_user uuid, p_device text)
RETURNS SETOF text
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = iam, public, pg_temp AS $$
  SELECT DISTINCT g.role
    FROM devices d
    JOIN nodes dn ON dn.id = d.node_id
    JOIN grants g ON g.user_id = p_user
                 AND g.revoked_at IS NULL
                 AND (g.expires_at IS NULL OR g.expires_at > now())
    JOIN nodes gn ON gn.id = g.node_id AND gn.path @> dn.path
    JOIN users u  ON u.id = g.user_id AND u.status = 'active'
   WHERE d.device_id = p_device
$$;

-- Every device the user can reach, one row per (device, role). Join your own tables
-- against this, or page with:
--   SELECT * FROM iam_api_v1.accessible_devices($1) WHERE device_id > $2 ORDER BY device_id LIMIT $3
CREATE FUNCTION iam_api_v1.accessible_devices(p_user uuid)
RETURNS TABLE (device_id text, role text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = iam, public, pg_temp AS $$
  SELECT DISTINCT d.device_id, g.role
    FROM grants g
    JOIN nodes gn  ON gn.id = g.node_id
    JOIN nodes dn  ON dn.path <@ gn.path
    JOIN devices d ON d.node_id = dn.id
    JOIN users u   ON u.id = g.user_id AND u.status = 'active'
   WHERE g.user_id = p_user
     AND g.revoked_at IS NULL
     AND (g.expires_at IS NULL OR g.expires_at > now())
$$;

-- ---------------------------------------------------------------------
-- Privileges
-- Writer: the private tables only. Reader: the contract only.
-- (New schemas grant nothing to PUBLIC, so no revokes are needed.)
-- ---------------------------------------------------------------------

GRANT USAGE ON SCHEMA iam TO iam_writer;
GRANT SELECT, INSERT, UPDATE, DELETE ON iam.nodes   TO iam_writer;
GRANT SELECT, INSERT, UPDATE         ON iam.users   TO iam_writer;   -- disable users, don't delete
GRANT SELECT, INSERT, UPDATE, DELETE ON iam.devices TO iam_writer;
GRANT SELECT, INSERT, UPDATE         ON iam.grants  TO iam_writer;   -- soft revoke, no delete
GRANT SELECT, INSERT                 ON iam.audit_log TO iam_writer; -- append-only
GRANT EXECUTE ON FUNCTION iam.move_node(uuid, uuid) TO iam_writer;

GRANT USAGE ON SCHEMA iam_api_v1 TO iam_reader;
GRANT SELECT ON iam_api_v1.meta, iam_api_v1.device_placement TO iam_reader;
GRANT EXECUTE ON FUNCTION
  iam_api_v1.user_context(text),
  iam_api_v1.scope(uuid),
  iam_api_v1.roles_on_device(uuid, text),
  iam_api_v1.accessible_devices(uuid)
TO iam_reader;
