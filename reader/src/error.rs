use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
}
