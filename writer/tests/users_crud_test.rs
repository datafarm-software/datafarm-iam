mod common;
use common::*;
use iam_writer::db::users::{NewUser, UpdateUser, UserStatus};
use iam_writer::error::Error;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;
use uuid::uuid;

#[sqlx::test(migrations = "../migrations", fixtures("nodes", "users"))]
async fn test_crud_user(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let writer = IamWriter::new(common::login_to_pool(&pool, "iam_writer").await).await?;
    //Retrieve non existent user returns Database Not Found
    let err = writer
        .get_user(&uuid!("12340000-0000-0000-0000-000000000001"))
        .await
        .expect_err("user should not exist");
    assert!(
        matches!(err, Error::Database(sqlx::Error::RowNotFound)),
        "expected a not-found error, got: {err:?}"
    );
    //Create
    let user_id = writer
        .insert_user(&NewUser {
            keycloak_sub: sub_division::user::DAN_SUB.to_owned(),
            email: sub_division::user::DAN_EMAIL.to_owned(),
            home_node_id: fixtures::node::CLIENT_A,
        })
        .await
        .expect("insert should return user id");
    //Retrieve
    {
        let user = writer.get_user(&user_id).await.expect("user should exist");
        assert_eq!(&user.id, &user_id);
        assert_eq!(user.keycloak_sub, sub_division::user::DAN_SUB);
        assert_eq!(user.email, sub_division::user::DAN_EMAIL);
        assert_eq!(user.home_node_id, fixtures::node::CLIENT_A);
        assert_eq!(user.status, UserStatus::Active);
    }
    //Update
    writer
        .update_user(&UpdateUser {
            id: &user_id,
            keycloak_sub: None,
            email: Some(fixtures::user::TOM_EMAIL.to_owned()),
            home_node_id: None,
            status: None,
        })
        .await
        .expect("update should return nothing if succeeded");
    //Deactivate user
    let user = writer
        .deactivate_user(&user_id)
        .await
        .expect("deactivated user should be returned");
    assert_eq!(&user.id, &user_id);
    assert_eq!(user.keycloak_sub, sub_division::user::DAN_SUB);
    assert_eq!(user.email, fixtures::user::TOM_EMAIL);
    assert_eq!(user.home_node_id, fixtures::node::CLIENT_A);
    assert_eq!(user.status, UserStatus::Disabled);
    Ok(())
}
