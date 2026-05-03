use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxRate {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    /// Rate in basis points (e.g. 1000 = 10.00%)
    pub rate_bps: i32,
    pub tax_type: String,
    pub is_default: bool,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaxRate {
    pub name: String,
    pub rate_bps: i32,
    #[serde(default = "default_tax_type")]
    pub tax_type: String,
    #[serde(default)]
    pub is_default: bool,
}

fn default_tax_type() -> String {
    "exclusive".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTaxRate {
    pub name: Option<String>,
    pub rate_bps: Option<i32>,
    pub tax_type: Option<String>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
}
