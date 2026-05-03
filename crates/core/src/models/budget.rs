use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub fiscal_year: i32,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLine {
    pub id: String,
    pub budget_id: String,
    pub account_id: String,
    pub month: i32,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBudget {
    pub name: String,
    pub fiscal_year: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateBudget {
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

/// One entry in a batch upsert for budget lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertBudgetLine {
    pub account_id: String,
    /// 1–12
    pub month: i32,
    pub amount: i64,
}

// ── Budget vs Actual ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetVsActualLine {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub month: i32,
    pub budgeted: i64,
    pub actual: i64,
    pub variance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetVsActualReport {
    pub budget_id: String,
    pub budget_name: String,
    pub fiscal_year: i32,
    pub lines: Vec<BudgetVsActualLine>,
    pub total_budgeted: i64,
    pub total_actual: i64,
    pub total_variance: i64,
}
