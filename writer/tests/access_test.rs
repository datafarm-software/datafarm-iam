mod common;
use common::*;
use iam_reader::IamReader;
use iam_writer::db::{Device, Grant, NewNode, NewUser, UserStatus};
use iam_writer::error::Error;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;
use uuid::uuid;

#[sqlx::test(
    migrations = "../migrations",
    fixtures("nodes", "users", "grants", "devices")
)]
async fn test_access_restrictions(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    let reader = IamReader::new(common::login_to_pool(&pool, "iam_reader").await).await?;
    // Create Sub Division's node
    let node_id = match writer
        .insert_node(&NewNode {
            parent_id: Some(fixtures::node::CLIENT_A),
            name: sub_division::node::SUB_DIVISION_NAME.to_owned(),
            tags: vec![sub_division::node::SUB_DIVISION_TAG.to_owned()],
            keycloak_org_id: None,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => panic!("insert_node: {}", e),
    };
    // Create Sub Division's user
    let user_id = writer
        .insert_user(&NewUser {
            keycloak_sub: sub_division::user::DAN_SUB.to_owned(),
            email: sub_division::user::DAN_EMAIL.to_owned(),
            home_node_id: node_id,
        })
        .await?;

    // Move Device to Sub Division.
    writer
        .move_device(&Device {
            device_id: fixtures::device::DEV_1001.to_owned(),
            node_id,
        })
        .await?;

    // Assert everyone has access to device, except Dan because no grant yet.
    {
        let device_ids: Vec<String> = reader.accessible_devices(fixtures::user::DF_USER).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> = reader.accessible_devices(fixtures::user::BOB).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> = reader.accessible_devices(fixtures::user::TOM).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> = reader.accessible_devices(user_id).await?;
        assert!(
            device_ids.is_empty(),
            "expected no accessible devices, got {device_ids:?}"
        )
    }
    // Grant Dan access to Sub Division devices
    writer
        .grant_user_access(&Grant {
            user_id,
            node_id,
            role: fixtures::role::MEMBER,
            granted_by: Some(fixtures::user::DF_USER),
        })
        .await?;
    {
        // Assert Dan now has access
        let device_ids: Vec<String> = reader.accessible_devices(user_id).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        // Assert DF User still has access
        let device_ids: Vec<String> = reader.accessible_devices(fixtures::user::DF_USER).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("nodes", "users"))]
async fn test_crud_user(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Retrieve non existent user returns Database Not Found
    {
        let err = writer
            .get_user(&uuid!("12340000-0000-0000-0000-000000000001"))
            .await
            .expect_err("user should not exist");
        assert!(
            matches!(err, Error::Database(sqlx::Error::RowNotFound)),
            "expected a not-found error, got: {err:?}"
        );
    }
    //Create
    let user_id = writer
        .insert_user(&NewUser {
            keycloak_sub: sub_division::user::DAN_SUB.to_owned(),
            email: sub_division::user::DAN_EMAIL.to_owned(),
            home_node_id: fixtures::node::CLIENT_A,
        })
        .await?;
    //Retrieve
    let user = writer.get_user(&user_id).await.expect("user should exist");
    assert_eq!(user.id, user_id);
    assert_eq!(user.keycloak_sub, sub_division::user::DAN_SUB);
    assert_eq!(user.email, sub_division::user::DAN_EMAIL);
    assert_eq!(user.home_node_id, fixtures::node::CLIENT_A);
    assert_eq!(user.status, UserStatus::Active);
    Ok(())
}
