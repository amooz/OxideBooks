use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub id: String,
    pub organization_id: String,
    pub entity_type: String,
    pub name: String,
    pub min_amount: Option<MinorUnits>,
    pub max_amount: Option<MinorUnits>,
    pub required_role: String,
    pub is_active: bool,
    pub sort_order: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateApprovalRule {
    pub entity_type: String,
    pub name: String,
    pub min_amount: Option<MinorUnits>,
    pub max_amount: Option<MinorUnits>,
    #[serde(default = "default_role")]
    pub required_role: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApprovalRule {
    pub name: Option<String>,
    pub min_amount: Option<MinorUnits>,
    pub max_amount: Option<MinorUnits>,
    pub required_role: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

fn default_role() -> String {
    "accountant".into()
}
