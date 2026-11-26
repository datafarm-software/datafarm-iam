use crate::db::{NewNode, NewUser};
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
        let mut conn = self.pool.acquire().await?;
        let node_id = sqlx::query_scalar(
            "INSERT INTO iam.nodes (parent_id, name, tags, keycloak_org_id)\
                VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(n.parent_id)
        .bind(&n.name)
        .bind(&n.tags)
        .bind(&n.keycloak_org_id)
        .fetch_one(conn.as_mut())
        .await?;
        Ok(node_id)
    }

    pub async fn insert_user(&self, u: &NewUser) -> Result<Uuid, Error> {
        let mut conn = self.pool.acquire().await?;
        let user_id = sqlx::query_scalar(
            "INSERT INTO iam.users (keycloak_sub, email, home_node_id)\
        VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(&u.keycloak_sub)
        .bind(&u.email)
        .bind(u.home_node_id)
        .fetch_one(conn.as_mut())
        .await?;
        Ok(user_id)
    }
}
