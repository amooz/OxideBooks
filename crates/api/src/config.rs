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
    /// Public base URL of this API, used for OAuth2 redirect URIs and SAML ACS URLs.
    /// E.g. "https://api.example.com"
    pub base_url: String,
    /// CORS allowed origins. Use ["*"] for development, explicit list for production.
    /// E.g. ["https://app.example.com"]
    pub allowed_origins: Vec<String>,
    /// Base URL for the exchange rate provider (Frankfurter-compatible API).
    pub exchange_rate_url: String,
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
            .set_default("auth.token_expiry_hours", 24)?
            .set_default("auth.refresh_expiry_days", 30)?
            .set_default("app.registration_open", true)?
            .set_default("app.default_currency", "USD")?
            .set_default("app.base_url", "http://localhost:3000")?
            .set_default("app.allowed_origins", vec!["*"])?
            .set_default("app.exchange_rate_url", "https://api.frankfurter.app")?
            .build()?;

        let settings: Self = cfg.try_deserialize()?;
        settings.validate()?;
        Ok(settings)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.database.url.starts_with("sqlite://") {
            anyhow::bail!(
                "OXIDEBOOKS__DATABASE__URL must be a PostgreSQL connection string \
                 (sqlite:// is not supported)"
            );
        }
        if self.database.url.is_empty() {
            anyhow::bail!("OXIDEBOOKS__DATABASE__URL must be set (PostgreSQL required)");
        }
        if self.auth.jwt_secret == "change-me" || self.auth.jwt_secret.len() < 32 {
            anyhow::bail!(
                "OXIDEBOOKS__AUTH__JWT_SECRET must be set to a random secret of at least \
                 32 characters"
            );
        }
        Ok(())
    }
}
