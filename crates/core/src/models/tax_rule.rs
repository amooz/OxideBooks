use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct TaxRule {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub country_code: String,
    pub region_code: Option<String>,
    pub tax_rate_id: String,
    pub applies_to: String,
    pub is_active: bool,
    pub priority: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxRule {
    pub name: String,
    pub country_code: String,
    pub region_code: Option<String>,
    pub tax_rate_id: String,
    #[serde(default = "default_applies_to")]
    pub applies_to: String,
    #[serde(default)]
    pub priority: i32,
}

fn default_applies_to() -> String {
    "sales".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxRule {
    pub name: Option<String>,
    pub tax_rate_id: Option<String>,
    pub applies_to: Option<String>,
    pub is_active: Option<bool>,
    pub priority: Option<i32>,
}

/// Suggested tax rate for a contact based on their address.
#[derive(Debug, Serialize)]
pub struct SuggestedTaxRate {
    pub tax_rate_id: Option<String>,
    pub tax_rate_name: Option<String>,
    pub rate_bps: Option<i32>,
    pub matched_rule_id: Option<String>,
}
