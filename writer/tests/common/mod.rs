pub mod fixtures;
pub mod sub_division;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn reader_pool(owner_pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE iam_reader").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(owner_pool.connect_options().as_ref().clone())
        .await
        .expect("reader pool")
}
