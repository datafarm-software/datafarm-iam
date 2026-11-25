use crate::error::Error;
use sqlx::{Connection, PgPool};
use uuid::Uuid;

pub struct IamReader {
    pool: PgPool,
}

impl IamReader {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let mut conn = pool.acquire().await?;
        conn.ping().await?;
        Ok(Self { pool })
    }

    pub async fn accessible_devices(&self, user_id: Uuid) -> Result<Vec<String>, Error> {
        let mut conn = self.pool.acquire().await?;
        let device_ids: Vec<String> =
            sqlx::query_scalar("SELECT device_id FROM iam_api_v1.accessible_devices($1)")
                .bind(user_id)
                .fetch_all(conn.as_mut())
                .await?;
        Ok(device_ids)
    }
}
