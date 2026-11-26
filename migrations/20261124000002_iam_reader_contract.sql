-- =====================================================================
-- 2 of 3: public read contract, iam_api_v1
--
--   iam_api_v1  the only schema downstream services (role iam_reader) can use.
--               Breaking changes ship as iam_api_v2; additive changes bump meta.minor.
--
-- Depends on migration 1 (reads the private iam tables).
-- Creating the group role needs CREATEROLE once; otherwise create it beforehand.
-- Ops step, outside these migrations (one login role per service):
--   CREATE ROLE charting_svc LOGIN PASSWORD '...' IN ROLE iam_reader;
--
-- Rule
--   * Contract functions return nothing for disabled users and revoked/expired grants.
--
-- Views run with the owner's rights; functions are SECURITY DEFINER with a pinned
-- search_path, so readers never need access to the iam schema.
-- =====================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'iam_reader') THEN
    CREATE ROLE iam_reader NOLOGIN;
  END IF;
END $$;

CREATE SCHEMA iam_api_v1;

CREATE VIEW iam_api_v1.meta AS
  SELECT 1 AS major, 0 AS minor;

-- Where every device sits. No personal data.
CREATE VIEW iam_api_v1.device_placement AS
  SELECT d.device_id, d.node_id, n.path::text AS node_path
    FROM iam.devices d
    JOIN iam.nodes n ON n.id = d.node_id;

-- Who is this token's subject? Zero rows = unknown user.
-- home_org_id = nearest ancestor-or-self org boundary of the home node: the tenant's
-- identifier in the identity provider, matching the org claim in the user's token.
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
-- Privileges: the reader gets the contract only.
-- ---------------------------------------------------------------------

GRANT USAGE ON SCHEMA iam_api_v1 TO iam_reader;
GRANT SELECT ON iam_api_v1.meta, iam_api_v1.device_placement TO iam_reader;
GRANT EXECUTE ON FUNCTION
  iam_api_v1.user_context(text),
  iam_api_v1.scope(uuid),
  iam_api_v1.roles_on_device(uuid, text),
  iam_api_v1.accessible_devices(uuid)
TO iam_reader;
