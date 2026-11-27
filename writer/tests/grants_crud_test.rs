mod common;
use common::*;
use iam_writer::db::grants::{NewGrant, Role};
use iam_writer::writer::IamWriter;
use sqlx::PgPool;

#[sqlx::test(
    migrations = "../migrations",
    fixtures("nodes", "users", "grants", "devices")
)]
async fn test_crud_grant(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Retrieve non existent returns Not Found
    //Create
    let grant_id = writer
        .grant_user_access(&NewGrant {
            user_id: &fixtures::user::TOM,
            node_id: &fixtures::node::CLIENT_B,
            role: Role::Member,
            granted_by: Some(&fixtures::user::DF_USER),
            expires_at: None,
        })
        .await?;

    //Read
    let grant = writer
        .get_grant(&grant_id)
        .await
        .expect("get grant should return grant");
    assert_eq!(grant.id, grant_id);
    assert_eq!(grant.user_id, fixtures::user::TOM);
    assert_eq!(grant.node_id, fixtures::node::CLIENT_B);
    assert_eq!(grant.role, Role::Member);
    assert_eq!(grant.granted_by, Some(fixtures::user::DF_USER));
    assert_eq!(grant.expires_at, None);
    assert_eq!(grant.revoked_at, None);
    Ok(())
}
