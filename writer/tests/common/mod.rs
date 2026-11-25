pub mod fixtures;
pub mod sub_division;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn login_to_pool(owner_pool: &PgPool, role: &str) -> PgPool {
    let role = role.to_owned();
    PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |conn, _| {
            let role = role.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('role', $1, false)")
                    .bind(role)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with(owner_pool.connect_options().as_ref().clone())
        .await
        .expect("login to pool")
}
