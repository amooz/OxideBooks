use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxGroupRate {
    pub id: String,
    pub group_id: String,
    pub tax_rate_id: String,
    pub tax_rate_name: String,
    pub rate: MinorUnits,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxGroup {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub combined_rate: MinorUnits,
    pub is_active: bool,
    pub rates: Vec<TaxGroupRate>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct TaxGroupRateInput {
    pub tax_rate_id: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxGroup {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub rates: Vec<TaxGroupRateInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxGroup {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub rates: Option<Vec<TaxGroupRateInput>>,
}
