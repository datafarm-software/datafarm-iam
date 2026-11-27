mod common;
use chrono::{DateTime, NaiveDate, Utc};
use common::*;
use iam_writer::db::grants::{NewGrant, Role, UpdateGrant};
use iam_writer::error::Error;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;
use uuid::uuid;

#[sqlx::test(
    migrations = "../migrations",
    fixtures("nodes", "users", "grants", "devices")
)]
async fn test_crud_grant(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let non_existent_grant_id = &uuid!("00000000-0000-0000-0000-000000000009");
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Retrieve non existent returns Not Found
    // Non Existent User
    let err = writer
        .get_user_grants(non_existent_grant_id)
        .await
        .expect_err("should throw error not found");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );
    // Non Existent Node
    let err = writer
        .get_user_grants(&sub_division::node::SUB_DIVISION)
        .await
        .expect_err("should throw error not found");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );
    // Non Existent Grant
    let err = writer
        .get_grant(&sub_division::node::SUB_DIVISION)
        .await
        .expect_err("should throw error not found");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );

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
    //Duplication returns Conflict
    let err = writer
        .grant_user_access(&NewGrant {
            user_id: &fixtures::user::TOM,
            node_id: &fixtures::node::CLIENT_B,
            role: Role::Member,
            granted_by: Some(&fixtures::user::DF_USER),
            expires_at: None,
        })
        .await
        .expect_err("duplicate grant should throw conflict");
    assert!(
        matches!(err, Error::Conflict(_)),
        "expected a conflict error, got: {err:?}"
    );

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

    let user_grants = writer
        .get_user_grants(&fixtures::user::TOM)
        .await
        .expect("should return vec grants");
    // Tom should now have two grants. One for Client A, one for Client B.
    assert_eq!(user_grants.len(), 2);

    // Tom's grant on Client_B
    let node_grants = writer
        .get_node_grants(&fixtures::node::CLIENT_B)
        .await
        .expect("should return vec grants");
    assert_eq!(node_grants.len(), 1);

    //Update
    let naive_dt = NaiveDate::from_ymd_opt(2030, 11, 27)
        .expect("Invalid date")
        .and_hms_opt(14, 30, 0)
        .expect("Invalid time");
    let expiry_date: DateTime<Utc> = naive_dt.and_utc();
    writer
        .update_grant(&UpdateGrant {
            id: &grant_id,
            expires_at: Some(&expiry_date),
        })
        .await
        .expect("update on existing grant should not return error");

    let err = writer
        .update_grant(&UpdateGrant {
            id: non_existent_grant_id,
            expires_at: None,
        })
        .await
        .expect_err("update on non existent grant should return error");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );

    //Delete = Revoke
    let err = writer
        .revoke_grant(non_existent_grant_id)
        .await
        .expect_err("revoke on non existent should return not found");
    assert!(
        matches!(err, Error::NotFound),
        "expected a not-found error, got: {err:?}"
    );

    let grant = writer
        .revoke_grant(&grant_id)
        .await
        .expect("delete should return grant");
    assert_eq!(grant.user_id, fixtures::user::TOM);
    assert_eq!(grant.node_id, fixtures::node::CLIENT_B);
    assert_eq!(grant.role, Role::Member);
    assert_eq!(grant.granted_by, Some(fixtures::user::DF_USER));
    assert_eq!(grant.expires_at, Some(expiry_date));
    Ok(())
}
