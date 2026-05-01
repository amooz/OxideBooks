pub mod error;
pub mod repos;

pub use error::DbError;
pub use sqlx::PgPool;

/// Embedded migrations — used both at runtime and by `#[sqlx::test]`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Run all pending migrations before the server starts accepting requests.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Open a PostgreSQL connection pool for the given `url`.
pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(url)
        .await
}
