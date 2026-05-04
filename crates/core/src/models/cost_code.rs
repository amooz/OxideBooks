use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct CostCode {
    pub id: String,
    pub organization_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub cost_type: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateCostCode {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_cost_type")]
    pub cost_type: String,
}

fn default_cost_type() -> String {
    "labor".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateCostCode {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cost_type: Option<String>,
    pub is_active: Option<bool>,
}
