use crate::error::Error;
use crate::writer::IamWriter;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Input for creating a node. id, path and created_at are filled in by the database
/// (gen_random_uuid(), the nodes_set_path trigger, now()), so they aren't here.
/// Reparenting an existing node goes through iam.move_node(), not an update of parent_id.
#[derive(Debug, Clone)]
pub struct NewNode {
    /// None = a root node.
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub tags: Vec<String>,
    /// Some(..) marks this node as a tenant (org) boundary.
    pub keycloak_org_id: Option<String>,
}

/// A full row of iam.nodes, as returned by `INSERT ... RETURNING` or a SELECT.
///
/// `path` is an ltree in Postgres, which sqlx can't decode, so every query that returns
/// it must cast: `path::text AS path`. Example column list:
///   id, parent_id, path::text AS path, name, tags, keycloak_org_id, created_at
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Node {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub path: String,
    pub name: String,
    pub tags: Vec<String>,
    pub keycloak_org_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpdateNode<'a> {
    pub id: &'a Uuid,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MoveNode<'a> {
    pub id: &'a Uuid,
    pub parent_id: &'a Uuid,
}

impl IamWriter {
    pub async fn insert_node(&self, n: &NewNode) -> Result<Node, Error> {
        let node = sqlx::query_as::<_, Node>(
            "INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id)\
                VALUES ($1, $2, $3, $4)
                RETURNING *, path::text AS path",
        )
        .bind(n.parent_id)
        .bind(&n.name)
        .bind(&n.tags)
        .bind(&n.keycloak_org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                Error::Conflict("an active grant for this user, node and role exists")
            }
            sqlx::Error::Database(db) if db.is_check_violation() => Error::NodeLimit,
            sqlx::Error::Database(db) if db.is_foreign_key_violation() => Error::NotFound,
            other => Error::Database(other),
        })?;
        Ok(node)
    }

    pub async fn update_node(&self, n: &UpdateNode<'_>) -> Result<Node, Error> {
        let node = sqlx::query_as::<_, Node>(
            "UPDATE iam.nodes
            SET name = COALESCE($2, name),
                tags = COALESCE($3, tags)
            WHERE id = $1
            RETURNING *, path::text AS path",
        )
        .bind(n.id)
        .bind(&n.name)
        .bind(&n.tags)
        .fetch_one(&self.pool)
        .await?;
        Ok(node)
    }

    pub async fn move_node(&self, n: &MoveNode<'_>) -> Result<String, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT iam.move_node($1, $2)")
            .bind(n.id)
            .bind(n.parent_id)
            .fetch_one(tx.as_mut())
            .await?;
        let path = sqlx::query_scalar("SELECT path::text AS path FROM iam.nodes WHERE id=$1")
            .bind(n.id)
            .fetch_one(tx.as_mut())
            .await?;
        let _res = tx.commit().await;
        // TODO: insert the 'node.move' audit row here, through tx.as_mut()
        Ok(path)
    }

    pub async fn get_node(&self, node_id: &Uuid) -> Result<Node, Error> {
        let node = sqlx::query_as::<_, Node>(
            "SELECT id, parent_id, path::text as path, name, tags, keycloak_org_id, created_at
            FROM iam.nodes WHERE id = $1",
        )
        .bind(node_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::NotFound,
            other => Error::Database(other),
        })?;

        Ok(node)
    }
}
