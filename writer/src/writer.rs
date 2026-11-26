use crate::db::{device::Device, grant::Grant};
use crate::error::Error;
use sqlx::{Connection, PgPool};

pub struct IamWriter {
    pub pool: PgPool,
}

impl IamWriter {
    pub async fn new(pool: PgPool) -> Result<Self, Error> {
        let mut conn = pool.acquire().await?;
        conn.ping().await?;
        Ok(Self { pool })
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
