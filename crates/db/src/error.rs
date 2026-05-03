use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("record not found")]
    NotFound,

    #[error("unique constraint violation: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("domain error: {0}")]
    Core(#[from] oxidebooks_core::CoreError),
}

impl DbError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, DbError::NotFound)
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self, DbError::Conflict(_))
    }
}

/// Map sqlx errors: translate unique-violation into DbError::Conflict.
pub fn map_sqlx_err(e: sqlx::Error) -> DbError {
    if let sqlx::Error::Database(ref db) = e {
        if db.kind() == sqlx::error::ErrorKind::UniqueViolation {
            return DbError::Conflict(db.message().to_string());
        }
    }
    DbError::Sqlx(e)
}
