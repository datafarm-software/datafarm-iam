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
