use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("not found")]
    NotFound,

    #[error("node limit reached")]
    NodeLimit,

    #[error("conflict: {0}")]
    Conflict(&'static str),
}
