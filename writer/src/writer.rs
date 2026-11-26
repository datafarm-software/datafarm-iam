use crate::error::Error;
use sqlx::{Connection, PgPool};

pub struct IamWriter {
    pool: PgPool,
}

impl IamWriter {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let mut conn = pool.acquire().await?;
        conn.ping().await?;
        Ok(Self { pool })
    }
}
