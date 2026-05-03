use serde::{Deserialize, Serialize};
use time::Date;

use crate::models::{date_serde, ProfitLossReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedProfitLoss {
    #[serde(with = "date_serde")]
    pub from: Date,
    #[serde(with = "date_serde")]
    pub to: Date,
    pub per_org: Vec<OrgProfitLoss>,
    pub combined_revenue: i64,
    pub combined_expenses: i64,
    pub combined_net_income: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgProfitLoss {
    pub org_id: String,
    pub report: ProfitLossReport,
}
