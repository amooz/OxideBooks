use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{models::date_serde, money::MinorUnits};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseClaimLine {
    pub id: String,
    pub claim_id: String,
    #[serde(with = "date_serde")]
    pub date: time::Date,
    pub description: String,
    pub amount: MinorUnits,
    pub category: Option<String>,
    pub receipt_url: Option<String>,
    pub account_id: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseClaim {
    pub id: String,
    pub organization_id: String,
    pub claimant_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub submitted_at: Option<OffsetDateTime>,
    pub reviewed_at: Option<OffsetDateTime>,
    pub reviewer_id: Option<String>,
    pub reviewer_notes: Option<String>,
    pub reimbursed_at: Option<OffsetDateTime>,
    pub currency_code: String,
    pub total_amount: MinorUnits,
    pub lines: Vec<ExpenseClaimLine>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExpenseClaimLine {
    #[serde(with = "date_serde")]
    pub date: time::Date,
    pub description: String,
    pub amount: MinorUnits,
    pub category: Option<String>,
    pub receipt_url: Option<String>,
    pub account_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExpenseClaim {
    pub claimant_id: String,
    pub title: String,
    pub description: Option<String>,
    pub currency_code: Option<String>,
    pub lines: Vec<CreateExpenseClaimLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExpenseClaim {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewExpenseClaim {
    pub notes: Option<String>,
}
