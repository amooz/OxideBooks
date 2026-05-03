use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpensePolicy {
    pub id: String,
    pub organization_id: String,
    pub category: String,
    pub max_amount: i64,
    pub requires_receipt_above: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertExpensePolicy {
    pub max_amount: i64,
    #[serde(default)]
    pub requires_receipt_above: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementLine {
    #[serde(with = "crate::models::date_serde")]
    pub date: time::Date,
    pub description: String,
    pub reference: Option<String>,
    pub debit: i64,
    pub credit: i64,
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactStatement {
    pub contact_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub from: time::Date,
    #[serde(with = "crate::models::date_serde")]
    pub to: time::Date,
    pub opening_balance: i64,
    pub lines: Vec<StatementLine>,
    pub closing_balance: i64,
}
