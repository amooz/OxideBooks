use std::net::SocketAddr;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use oxidebooks_api::{config::Settings, routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,oxidebooks=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::load()?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        db = %settings.database.url,
        "starting OxideBooks"
    );

    let pool = oxidebooks_db::connect(&settings.database.url).await?;
    oxidebooks_db::run_migrations(&pool).await?;

    info!("database migrations applied");

    let app_state = AppState::new(pool, settings.clone());
    let app = routes::build(app_state);

    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port).parse()?;

    info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
