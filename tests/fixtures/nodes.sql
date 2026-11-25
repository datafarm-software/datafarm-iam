  INSERT INTO iam.nodes (id, parent_id, name, tags)
    VALUES ('00000000-0000-0000-0000-000000000001', NULL, 'DataFarm', '{datafarm}');

  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES ('00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000001',
        'CompanyA', '{vendor}', 'seed-org-companya', 'seed:companya');

  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id, external_ref)
    VALUES ('00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000002',
        'ClientA', '{client}', 'seed-org-clienta', 'seed:clienta');
