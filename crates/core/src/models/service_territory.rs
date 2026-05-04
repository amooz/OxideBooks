use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct ServiceTerritory {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub region_code: Option<String>,
    pub country_code: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceTerritory {
    pub name: String,
    pub description: Option<String>,
    pub region_code: Option<String>,
    #[serde(default = "default_country")]
    pub country_code: String,
}

fn default_country() -> String {
    "US".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceTerritory {
    pub name: Option<String>,
    pub description: Option<String>,
    pub region_code: Option<String>,
    pub country_code: Option<String>,
    pub is_active: Option<bool>,
}
