  INSERT INTO iam.nodes (id, parent_id, name, tags)
    VALUES ('00000000-0000-0000-0000-000000000001', NULL, 'DataFarm', '{datafarm}');

  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000001',
        'CompanyA', '{vendor}', 'seed-org-companya');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000002',
        'ClientA', '{client}', 'seed-org-clienta');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000004','00000000-0000-0000-0000-000000000003',
        'SubClient1', '{client}', 'seed-org-sub-client1');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000005','00000000-0000-0000-0000-000000000004',
        'SubClient2', '{client}', 'seed-org-sub-client2');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000006','00000000-0000-0000-0000-000000000005',
        'SubClient3', '{client}', 'seed-org-sub-client3');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000007','00000000-0000-0000-0000-000000000006',
        'SubClient4', '{client}', 'seed-org-sub-client4');
  INSERT INTO iam.nodes (id, parent_id, name, tags, keycloak_org_id)
    VALUES ('00000000-0000-0000-0000-000000000008','00000000-0000-0000-0000-000000000007',
        'SubClient5', '{client}', 'seed-org-sub-client5');
