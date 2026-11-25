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
-- There is no separate "network user" concept. A network that receives a client's data
-- uses credentials issued for that client, so it is an ordinary user in this tree.
-- =====================================================================

DO $$
DECLARE
  v_datafarm   uuid;
  v_companya   uuid;
  v_clienta  uuid;

  v_geraud uuid;
  v_bob  uuid;
  v_tom uuid;
BEGIN
  -- ---------------------------------------------------------------- nodes
  INSERT INTO iam.nodes (name, tags)
    VALUES ('DataFarm', '{datafarm}') RETURNING id INTO v_datafarm;

  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_datafarm, 'CompanyA', '{vendor}', 'seed-org-companya', 'seed:companya')
    RETURNING id INTO v_companya;

  INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES (v_companya, 'ClientA', '{client}', 'seed-org-clienta', 'seed:clienta')
    RETURNING id INTO v_clienta;

  -- --------------------------------------------------------------- devices
  INSERT INTO iam.devices (device_id, node_id) VALUES
    ('dev-1001', v_clienta),

  -- ----------------------------------------------------------------- users
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-geraud', 'geraud@datafarm.co.za', v_datafarm) RETURNING id INTO v_geraud;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-bob', 'bob@companya.com', v_companya) RETURNING id INTO v_bob;
  INSERT INTO iam.users (keycloak_sub, email, home_node_id) VALUES
    ('seed-sub-tom', 'tom@clienta.com', v_rcl) RETURNING id INTO v_tom;

  -- ---------------------------------------------------------------- grants
  -- granted_by follows the delegation chain; NULL = system / bootstrap
  INSERT INTO iam.grants (user_id, node_id, role, granted_by, expires_at) VALUES
    (v_geraud, v_datafarm,    'admin',   NULL,    NULL),                       
    (v_bob,  v_companya,   'admin',   v_geraud, NULL),                       
    (v_tom, v_clienta, 'manager', v_geraud,  NULL),                       
END $$;
