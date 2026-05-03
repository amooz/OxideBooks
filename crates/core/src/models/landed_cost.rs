use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandedCostAllocation {
    pub id: String,
    pub landed_cost_id: String,
    pub grn_line_id: String,
    pub allocated_amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandedCost {
    pub id: String,
    pub organization_id: String,
    pub grn_id: String,
    pub description: String,
    pub amount: i64,
    pub allocation_method: String,
    pub currency: String,
    pub vendor_id: Option<String>,
    pub allocations: Vec<LandedCostAllocation>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLandedCost {
    pub description: String,
    pub amount: i64,
    #[serde(default = "default_allocation_method")]
    pub allocation_method: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub vendor_id: Option<String>,
}

fn default_allocation_method() -> String {
    "quantity".into()
}

fn default_currency() -> String {
    "USD".into()
}
