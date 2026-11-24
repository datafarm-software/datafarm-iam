-- =====================================================================
-- seed_example.sql: example data for exploring the IAM schema. NOT a migration
-- (kept out of the numbered migrations so sqlx never applies it by accident).
--
-- Run in the Query Tool attached to the `iam` database, as the schema owner or postgres.
-- Replace the 'seed-sub-*' values with real Keycloak subs when you connect real logins.
--
-- To reset (order matters because of foreign keys):
--   DELETE FROM iam.grants; DELETE FROM iam.devices; DELETE FROM iam.users; DELETE FROM iam.nodes;
--
-- Two example structures, deliberately different depths:
--
--   Platform
--   |- CompanyA (vendor)
--   |  `- DistributorA
--   |     `- GreenValley Farms (client)
--   |        |- North Area
--   |        |  |- Smith Farm (access-group, leaf)   dev-1001, dev-1002
--   |        |  `- (devices directly on the area)    dev-1003
--   |        `- South Area                           dev-1004, dev-1005
--   `- CompanyB (vendor)
--      `- BlueRiver Co-op (client, flat: devices sit directly on the client)
--                                                    dev-2001, dev-2002
--
-- There is no separate "network user" concept. A network that receives a client's data
-- uses credentials issued for that client, so it is an ordinary user in this tree.
-- =====================================================================

DO $$
DECLARE
  v_platform   uuid;
  v_companya   uuid;
  v_distrib_a  uuid;
  v_greenvalley uuid;
  v_north      uuid;
  v_smith      uuid;
  v_south      uuid;
  v_companyb   uuid;
  v_blueriver  uuid;

  v_alice uuid;  -- CompanyA admin
  v_dave  uuid;  -- DistributorA admin
  v_maria uuid;  -- GreenValley team manager
  v_ned   uuid;  -- North Area member
  v_sam   uuid;  -- grower, Smith Farm only
  v_sue   uuid;  -- support staff with an EXPIRED grant
  v_eve   uuid;  -- disabled user who still has a grant
  v_bob   uuid;  -- BlueRiver admin (CompanyB)
BEGIN
  -- ---------------------------------------------------------------- nodes
  INSERT INTO iam.nodes (name, tags)
    VALUES ('Platform', '{platform}') RETURNING id INTO v_platform;

  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_platform, 'CompanyA', '{vendor}', 'seed-org-companya', 'seed:companya')
    RETURNING id INTO v_companya;
  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_companya, 'DistributorA', '{distributor}', 'seed-org-distributora', 'seed:distributora')
    RETURNING id INTO v_distrib_a;
  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_distrib_a, 'GreenValley Farms', '{client}', 'seed-org-greenvalley', 'seed:greenvalley')
    RETURNING id INTO v_greenvalley;
  INSERT INTO iam.nodes (parent_id, name, tags)
    VALUES (v_greenvalley, 'North Area', '{area}') RETURNING id INTO v_north;
  INSERT INTO iam.nodes (parent_id, name, tags)
    VALUES (v_north, 'Smith Farm', '{access-group}') RETURNING id INTO v_smith;
  INSERT INTO iam.nodes (parent_id, name, tags)
    VALUES (v_greenvalley, 'South Area', '{area}') RETURNING id INTO v_south;

  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_platform, 'CompanyB', '{vendor}', 'seed-org-companyb', 'seed:companyb')
    RETURNING id INTO v_companyb;
  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_companyb, 'BlueRiver Co-op', '{client}', 'seed-org-blueriver', 'seed:blueriver')
    RETURNING id INTO v_blueriver;

  -- --------------------------------------------------------------- devices
  INSERT INTO iam.devices (device_id, node_id) VALUES
    ('dev-1001', v_smith),
    ('dev-1002', v_smith),
    ('dev-1003', v_north),
    ('dev-1004', v_south),
    ('dev-1005', v_south),
    ('dev-2001', v_blueriver),
    ('dev-2002', v_blueriver);

  -- ----------------------------------------------------------------- users
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-alice', 'alice@companya.example', v_companya) RETURNING id INTO v_alice;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-dave', 'dave@distributora.example', v_distrib_a) RETURNING id INTO v_dave;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-maria', 'maria@greenvalley.example', v_greenvalley) RETURNING id INTO v_maria;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-ned', 'ned@greenvalley.example', v_greenvalley) RETURNING id INTO v_ned;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-sam', 'sam@smithfarm.example', v_greenvalley) RETURNING id INTO v_sam;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-sue', 'sue@companya.example', v_companya) RETURNING id INTO v_sue;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id, status) VALUES
    ('seed-sub-eve', 'eve@greenvalley.example', v_greenvalley, 'disabled') RETURNING id INTO v_eve;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-bob', 'bob@blueriver.example', v_blueriver) RETURNING id INTO v_bob;

  -- ---------------------------------------------------------------- grants
  -- granted_by follows the delegation chain; NULL = system / bootstrap
  INSERT INTO iam.grants (user_id, node_id, role, granted_by, expires_at) VALUES
    (v_alice, v_companya,    'admin',   NULL,    NULL),                       -- everything under CompanyA
    (v_dave,  v_distrib_a,   'admin',   v_alice, NULL),                       -- DistributorA and its clients
    (v_maria, v_greenvalley, 'manager', v_dave,  NULL),                       -- both areas
    (v_ned,   v_north,       'member',  v_maria, NULL),                       -- North Area incl. Smith Farm
    (v_sam,   v_smith,       'member',  v_maria, NULL),                       -- ONLY Smith Farm stations
    (v_sue,   v_greenvalley, 'manager', v_alice, now() - interval '1 day'),   -- expired: sees nothing
    (v_eve,   v_south,       'member',  v_maria, NULL),                       -- user disabled: sees nothing
    (v_bob,   v_blueriver,   'admin',   NULL,    NULL);                       -- separate vendor tree
END $$;

-- =====================================================================
-- THINGS TO TRY (uncomment and run one at a time)
-- =====================================================================

-- The whole tree, indented by depth:
-- SELECT repeat('  ', nlevel(path) - 1) || name AS node, tags, path::text
--   FROM iam.nodes ORDER BY path;

-- Who is ned, and which org does he belong to?
-- SELECT * FROM iam_api_v1.user_context('seed-sub-ned');

-- What each person can reach (compare ned, sam, maria, dave, bob; then sue and eve):
-- SELECT * FROM iam_api_v1.accessible_devices(
--   (SELECT user_id FROM iam_api_v1.user_context('seed-sub-sam')));

-- Compact scope a service could cache:
-- SELECT * FROM iam_api_v1.scope(
--   (SELECT user_id FROM iam_api_v1.user_context('seed-sub-maria')));

-- Single-device check (expect a row for sam on dev-1001, nothing on dev-1003):
-- SELECT * FROM iam_api_v1.roles_on_device(
--   (SELECT user_id FROM iam_api_v1.user_context('seed-sub-sam')), 'dev-1001');

-- Move Smith Farm under South Area, then re-run ned's and sam's device lists:
-- SELECT iam.move_node(
--   (SELECT id FROM iam.nodes WHERE name = 'Smith Farm'),
--   (SELECT id FROM iam.nodes WHERE name = 'South Area'));

-- Try the boundaries as the restricted roles:
-- SET ROLE iam_reader;  SELECT * FROM iam.nodes;  RESET ROLE;   -- permission denied
