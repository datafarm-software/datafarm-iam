mod common;
use common::*;
use iam_writer::db::nodes::{MoveNode, NewNode, UpdateNode};
use iam_writer::error::Error;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;
use uuid::uuid;

#[sqlx::test(migrations = "../migrations", fixtures("nodes"))]
async fn test_crud_node(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Retrieve non existent node returns Not Found
    let err = writer
        .get_node(&uuid!("12340000-0000-0000-0000-000000000001"))
        .await
        .expect_err("node should not exist");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );
    //Create
    let node = writer
        .insert_node(&NewNode {
            parent_id: Some(fixtures::node::CLIENT_A),
            name: sub_division::node::SUB_DIVISION_NAME.to_owned(),
            tags: vec![sub_division::node::SUB_DIVISION_TAG.to_owned()],
            keycloak_org_id: Some("org id".to_owned()),
        })
        .await
        .expect("insert should return node");
    // Duplicate org id returns conflict. Some(org_id) signifies org root node.
    let err = writer
        .insert_node(&NewNode {
            parent_id: Some(fixtures::node::CLIENT_B),
            name: "Some New Name".to_owned(),
            tags: vec![sub_division::node::SUB_DIVISION_TAG.to_owned()],
            keycloak_org_id: Some("org id".to_owned()),
        })
        .await
        .expect_err("duplicate org id should return err");
    assert!(
        matches!(err, Error::Conflict(_)),
        "expected conflict, got {err:?}"
    );
    {
        //Retrieve
        let node = writer
            .get_node(&node.id)
            .await
            .expect("get should return node");
        assert_eq!(node.id, node_id);
        assert_eq!(node.parent_id, Some(fixtures::node::CLIENT_A));
        assert_eq!(node.name, sub_division::node::SUB_DIVISION_NAME);
        assert_eq!(
            node.tags,
            vec![sub_division::node::SUB_DIVISION_TAG.to_owned()]
        );
        assert_eq!(
            node.path,
            fixtures::ltree_path(&[
                fixtures::node::DATAFARM,
                fixtures::node::COMPANY_A,
                fixtures::node::CLIENT_A,
                node.id.clone(),
            ])
        );
    }
    {
        //Update
        let u_node = writer
            .update_node(&UpdateNode {
                id: &node.id,
                name: "New Name".to_owned(),
                tags: vec!["new-tag".to_owned()],
            })
            .await
            .expect("update should return node");
        assert_eq!(u_node.name, "New Name".to_owned());
        assert_eq!(u_node.tags, vec!["new-tag".to_owned()]);
    }
    let path = writer
        .move_node(&MoveNode {
            id: &node.id,
            parent_id: &fixtures::node::CLIENT_B,
        })
        .await
        .expect("move should return new path");
    assert_eq!(
        move_node.path,
        fixtures::ltree_path(&[
            fixtures::node::DATAFARM,
            fixtures::node::COMPANY_A,
            fixtures::node::CLIENT_B,
            node.id.clone(),
        ])
    );
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("nodes_limit"))]
async fn test_crud_node_limit(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Create
    let err = writer
        .insert_node(&NewNode {
            parent_id: Some(uuid!("00000000-0000-0000-0000-000000000008")),
            name: sub_division::node::SUB_DIVISION_NAME.to_owned(),
            tags: vec![sub_division::node::SUB_DIVISION_TAG.to_owned()],
            keycloak_org_id: Some("valid org id".to_owned()),
        })
        .await
        .expect_err("should not be allowed to create node deeper than level 8");
    assert!(
        matches!(err, Error::NodeLimit),
        "expected a node-limit error, got: {err:?}"
    );
    Ok(())
}
