use crate::error::Error;
use sqlx::{Connection, PgPool};

pub struct IamReader {
    pool: PgPool,
}

impl IamReader {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let mut conn = pool.acquire().await?;
        conn.ping().await?;
        Ok(Self { pool })
    }
}
