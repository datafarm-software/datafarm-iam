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

    // pub async fn insert_node(&self) -> Result<Uuid, Error> {
    //     sqlx::query("INSERT INTO iam.nodes (parent_id, name, tags) VALUES ($1, $2, $3, $4)")
    //         .bind(fixtures::node::CLIENT_A)
    //         .bind(sub_division::node::SUB_DIVISION_NAME)
    //         .bind(vec![sub_division::node::SUB_DIVISION_TAG.to_owned()])
    //         .execute(conn.as_mut())
    //         .await?;
    // }
}
