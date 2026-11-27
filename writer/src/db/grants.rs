use crate::error::Error;
use crate::writer::IamWriter;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Mirrors CHECK (role IN ('admin', 'manager', 'member')) on iam.grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Role {
    Admin,
    Manager,
    Member,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Manager => "manager",
            Role::Member => "member",
        }
    }
}

/// Input for creating a grant. id and granted_at are filled in by the database.
/// `granted_by` None = system / bootstrap. It is also recorded as the audit actor.
#[derive(Debug, Clone)]
pub struct NewGrant<'a> {
    pub user_id: &'a Uuid,
    pub node_id: &'a Uuid,
    pub role: Role,
    pub granted_by: Option<&'a Uuid>,
    /// None = never expires.
    pub expires_at: Option<DateTime<Utc>>,
}

/// The only editable part of a grant is its expiry. (user, node, role) is the grant's
/// identity: to change any of those, revoke it and create a new one.
#[derive(Debug, Clone)]
pub struct UpdateGrant<'a> {
    pub id: &'a Uuid,
    /// The new expiry. None = never expires (this sets the column to NULL).
    pub expires_at: Option<DateTime<Utc>>,
}

/// A full row of iam.grants.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Grant {
    pub id: Uuid,
    pub user_id: Uuid,
    pub node_id: Uuid,
    pub role: Role,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl IamWriter {
    pub async fn grant_user_access(&self, g: &NewGrant<'_>) -> Result<Uuid, Error> {
        sqlx::query_scalar(
            "INSERT INTO iam.grants (user_id, node_id, role, granted_by)\
        VALUES ($1, $2, $3, $4)
        RETURNING id",
        )
        .bind(g.user_id)
        .bind(g.node_id)
        .bind(g.role)
        .bind(g.granted_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                Error::Conflict("an active grant for this user, node and role exists")
            }
            sqlx::Error::Database(db) if db.is_foreign_key_violation() => Error::NotFound,
            other => Error::Database(other),
        })
    }

    pub async fn get_grant(&self, grant_id: &Uuid) -> Result<Grant, Error> {
        match sqlx::query_as::<_, Grant>(
            "SELECT id, user_id, node_id, role, granted_by, granted_at, expires_at, revoked_at
            FROM iam.grants WHERE id = $1",
        )
        .bind(grant_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(grant) => Ok(grant),
            Err(sqlx::Error::RowNotFound) => Err(Error::NotFound),
            Err(e) => Err(Error::Database(e)),
        }
    }

    pub async fn get_user_grants(&self, user_id: &Uuid) -> Result<Vec<Grant>, Error> {
        self.get_attached_grants("user_id", user_id).await
    }

    pub async fn get_node_grants(&self, node_id: &Uuid) -> Result<Vec<Grant>, Error> {
        self.get_attached_grants("node_id", node_id).await
    }

    pub async fn get_attached_grants(
        &self,
        query_filter: &str,
        id: &Uuid,
    ) -> Result<Vec<Grant>, Error> {
        let query = format!(
            "SELECT id, user_id, node_id, role, granted_by, granted_at, expires_at, revoked_at
            FROM iam.grants WHERE {query_filter} = $1"
        );
        let grants = sqlx::query_as::<_, Grant>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_all(&self.pool)
            .await?;
        if grants.is_empty() {
            Err(Error::NotFound)
        } else {
            Ok(grants)
        }
    }
}
