pub mod error;
pub mod repos;

pub use error::DbError;
pub use sqlx::PgPool;

/// Run all pending SQLx migrations embedded from `./migrations/`.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Open a PostgreSQL connection pool for the given `url`.
pub async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(url)
        .await
}
