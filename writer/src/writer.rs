use crate::db::{
    device::Device,
    grant::Grant,
    node::{MoveNode, NewNode, Node, UpdateNode},
    user::{NewUser, UpdateUser, User},
};
use crate::error::Error;
use sqlx::{Connection, PgPool};
use uuid::Uuid;

pub struct IamWriter {
    pool: PgPool,
}

impl IamWriter {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let mut conn = pool.acquire().await?;
        conn.ping().await?;
        Ok(Self { pool })
    }

    pub async fn insert_node(&self, n: &NewNode) -> Result<Uuid, Error> {
        let node_id = sqlx::query_scalar(
            "INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id)\
                VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(n.parent_id)
        .bind(&n.name)
        .bind(&n.tags)
        .bind(&n.keycloak_org_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(node_id)
    }

    pub async fn update_node(&self, n: &UpdateNode<'_>) -> Result<Node, Error> {
        let user = sqlx::query_as::<_, Node>(
            "UPDATE iam.nodes
            SET name = COALESCE($2, name),
                tags = COALESCE($3, tags)
            WHERE id = $1
            RETURNING id, parent_id, path::text as path, name, tags, keycloak_org_id, created_at",
        )
        .bind(n.id)
        .bind(&n.name)
        .bind(&n.tags)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn move_node(&self, n: &MoveNode<'_>) -> Result<Node, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT iam.move_node($1, $2)")
            .bind(n.id)
            .bind(n.parent_id)
            .execute(&mut *tx)
            .await?;
        let node = sqlx::query_as::<_, Node>(
            "SELECT id, parent_id, path::text AS path, name, tags, keycloak_org_id, created_at
           FROM iam.nodes
          WHERE id = $1",
        )
        .bind(n.id)
        .fetch_one(tx.as_mut())
        .await?;
        // TODO: insert the 'node.move' audit row here, through tx.as_mut()
        tx.commit().await?;
        Ok(node)
    }
    pub async fn get_node(&self, node_id: &Uuid) -> Result<Node, Error> {
        let node = sqlx::query_as::<_, Node>(
            "SELECT id, parent_id, path::text as path, name, tags, keycloak_org_id, created_at
            FROM iam.nodes WHERE id = $1",
        )
        .bind(node_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(node)
    }

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

    pub async fn move_device(&self, d: &Device) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("UPDATE iam.devices SET node_id = $1 WHERE device_id = $2")
            .bind(d.node_id)
            .bind(&d.device_id)
            .execute(conn.as_mut())
            .await?;
        Ok(())
    }

    pub async fn grant_user_access(&self, g: &Grant) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query(
            "INSERT INTO iam.grants (user_id, node_id, role, granted_by)\
        VALUES ($1, $2, $3, $4)",
        )
        .bind(g.user_id)
        .bind(g.node_id)
        .bind(g.role)
        .bind(g.granted_by)
        .execute(conn.as_mut())
        .await?;
        Ok(())
    }
}
