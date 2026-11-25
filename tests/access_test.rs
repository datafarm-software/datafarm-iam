mod common;

use sqlx::{Executor, PgPool};

#[sqlx::test(fixtures("nodes", "users", "grants", "devices"))]
async fn test_access_restrictions(pool: PgPool) -> sqlx::Result<()> {
    let mut conn = pool.acquire().await?;
    conn.execute(format!(
        "INSERT INTO iam.nodes (parent_id, name, tags) \
        VALUES ((SELECT id FROM iam.nodes WHERE name = '{0}'), )",
        common::node::CLIENT_A_NAME
    ))
    .await?;
    Ok(())
}
