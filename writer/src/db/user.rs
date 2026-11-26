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

/// A full row of iam.users, as returned by `INSERT ... RETURNING` or a SELECT.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub keycloak_sub: String,
    pub email: String,
    pub home_node_id: Uuid,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}
