mod common;
use common::*;
use iam_reader::IamReader;
use iam_writer::db::NewNode;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;

#[sqlx::test(
    migrations = "../migrations",
    fixtures("nodes", "users", "grants", "devices")
)]
async fn test_access_restrictions(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.acquire().await?;
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam").await).await?;
    let reader = IamReader::new(common::login_to_pool(&pool, "iam_reader").await).await?;
    // Create Sub Division's node
    let node_id = match writer
        .insert_node(&NewNode {
            parent_id: Some(sub_division::node::SUB_DIVISION),
            name: sub_division::node::SUB_DIVISION_NAME.to_owned(),
            tags: vec![sub_division::node::SUB_DIVISION_TAG.to_owned()],
            org_ref: None,
        })
        .await
    {
        Ok(nid) => nid,
        Err(e) => panic!("insert_node: {}", e),
    };
    // Create Sub Division's user
    sqlx::query(
        "INSERT INTO iam.users (id, keycloak_sub, email, home_node_id)\
        VALUES ($1, $2, $3, $4)",
    )
    .bind(sub_division::user::DAN)
    .bind(sub_division::user::DAN_SUB)
    .bind(sub_division::user::DAN_EMAIL)
    .bind(sub_division::node::SUB_DIVISION)
    .execute(conn.as_mut())
    .await?;

    // Move Device to Sub Division.
    sqlx::query("UPDATE iam.devices SET node_id = $1 WHERE device_id = $2")
        .bind(sub_division::node::SUB_DIVISION)
        .bind(fixtures::device::DEV_1001)
        .execute(conn.as_mut())
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
        let device_ids: Vec<String> = reader.accessible_devices(sub_division::user::DAN).await?;
        assert!(
            device_ids.is_empty(),
            "expected no accessible devices, got {device_ids:?}"
        )
    }
    // Grant Sub Division access to device
    sqlx::query(
        "INSERT INTO iam.grants (user_id, node_id, role, granted_by)\
        VALUES ($1, $2, $3, $4)",
    )
    .bind(sub_division::user::DAN)
    .bind(sub_division::node::SUB_DIVISION)
    .bind(fixtures::role::MEMBER)
    .bind(fixtures::user::DF_USER)
    .execute(conn.as_mut())
    .await?;
    {
        // Assert Dan now has access
        let device_ids: Vec<String> = reader.accessible_devices(sub_division::user::DAN).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        // Assert DF User still has access
        let device_ids: Vec<String> = reader.accessible_devices(fixtures::user::DF_USER).await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    Ok(())
}
