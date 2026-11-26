use crate::error::Error;
use crate::writer::IamWriter;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Mirrors the CHECK (status IN ('active', 'disabled')) on iam.users.
/// Stored as plain text, so sqlx maps the variants to/from the lowercase strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Disabled,
}

/// Input for creating a user. id, status and created_at are filled in by the
/// database (gen_random_uuid(), DEFAULT 'active', now()), so they aren't here.
/// Disabling a user later is an update, not part of creation.
#[derive(Debug, Clone)]
pub struct NewUser {
    pub keycloak_sub: String,
    pub email: String,
    pub home_node_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct UpdateUser<'a> {
    pub id: &'a Uuid,
    pub keycloak_sub: Option<String>,
    pub email: Option<String>,
    pub home_node_id: Option<Uuid>,
    pub status: Option<UserStatus>,
}

/// A full row of iam.users, as returned by `INSERT ... RETURNING` or a SELECT.
#[derive(Debug, Clone, Eq, PartialEq, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub keycloak_sub: String,
    pub email: String,
    pub home_node_id: Uuid,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}

impl IamWriter {
    pub async fn insert_user(&self, u: &NewUser) -> Result<Uuid, Error> {
        let user_id = sqlx::query_scalar(
            "INSERT INTO iam.users (keycloak_sub, email, home_node_id)\
        VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(&u.keycloak_sub)
        .bind(&u.email)
        .bind(u.home_node_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user_id)
    }

    pub async fn get_user(&self, user_id: &Uuid) -> Result<User, Error> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, keycloak_sub, email, home_node_id, status, created_at
           FROM iam.users
          WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn update_user(&self, u: &UpdateUser<'_>) -> Result<User, Error> {
        let user = sqlx::query_as::<_, User>(
            "UPDATE iam.users
            SET keycloak_sub = COALESCE($2, keycloak_sub),
                email        = COALESCE($3, email),
                home_node_id = COALESCE($4, home_node_id),
                status       = COALESCE($5, status)
            WHERE id = $1
            RETURNING id, keycloak_sub, email, home_node_id, status, created_at",
        )
        .bind(u.id)
        .bind(&u.keycloak_sub)
        .bind(&u.email)
        .bind(u.home_node_id)
        .bind(u.status)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn deactivate_user(&self, user_id: &Uuid) -> Result<User, Error> {
        let user = sqlx::query_as::<_, User>(
            "UPDATE iam.users
            SET status = 'disabled'
            WHERE id = $1
            RETURNING id, keycloak_sub, email, home_node_id, status, created_at",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }
}
