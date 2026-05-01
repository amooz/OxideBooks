use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub auth: AuthSettings,
    pub app: AppSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub token_expiry_hours: i64,
    pub refresh_expiry_days: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub registration_open: bool,
    pub default_currency: String,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::Environment::with_prefix("OXIDEBOOKS")
                    .separator("__")
                    .try_parsing(true),
            )
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("database.url", "sqlite://oxidebooks.db?mode=rwc")?
            .set_default("auth.jwt_secret", "change-me")?
            .set_default("auth.token_expiry_hours", 24)?
            .set_default("auth.refresh_expiry_days", 30)?
            .set_default("app.registration_open", true)?
            .set_default("app.default_currency", "USD")?
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}
