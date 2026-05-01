use std::sync::Arc;

use oxidebooks_db::PgPool;

use crate::config::Settings;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Settings>,
}

impl AppState {
    pub fn new(db: PgPool, config: Settings) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}
