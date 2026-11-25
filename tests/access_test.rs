mod common;
use common::*;

use sqlx::PgPool;

#[sqlx::test(fixtures("nodes", "users", "grants", "devices"))]
async fn test_access_restrictions(pool: PgPool) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;
    // Create Sub Division's node
    sqlx::query("INSERT INTO iam.nodes (id, parent_id, name, tags) VALUES ($1, $2, $3, $4)")
        .bind(sub_division::node::SUB_DIVISION)
        .bind(fixtures::node::CLIENT_A)
        .bind(sub_division::node::SUB_DIVISION_NAME)
        .bind(vec![sub_division::node::SUB_DIVISION_TAG.to_owned()])
        .execute(conn.as_mut())
        .await?;
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
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(fixtures::user::DF_USER)
                .fetch_all(conn.as_mut())
                .await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(fixtures::user::BOB)
                .fetch_all(conn.as_mut())
                .await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(fixtures::user::TOM)
                .fetch_all(conn.as_mut())
                .await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(sub_division::user::DAN)
                .fetch_all(conn.as_mut())
                .await?;
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
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(sub_division::user::DAN)
                .fetch_all(conn.as_mut())
                .await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    {
        // Assert DF User still has access
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(fixtures::user::DF_USER)
                .fetch_all(conn.as_mut())
                .await?;
        assert_eq!(device_ids, vec![fixtures::device::DEV_1001]);
    }
    Ok(())
}
