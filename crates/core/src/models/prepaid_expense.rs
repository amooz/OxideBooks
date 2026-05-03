use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaidExpenseEntry {
    pub id: String,
    pub schedule_id: String,
    #[serde(with = "date_serde")]
    pub period_date: Date,
    pub amount: i64,
    pub journal_entry_id: Option<String>,
    pub recognized_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaidExpenseSchedule {
    pub id: String,
    pub organization_id: String,
    pub description: String,
    pub total_amount: i64,
    pub asset_account_id: String,
    pub expense_account_id: String,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    pub frequency: String,
    pub is_active: bool,
    pub amortized_amount: i64,
    pub entries: Vec<PrepaidExpenseEntry>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePrepaidExpenseSchedule {
    pub description: String,
    pub total_amount: i64,
    pub asset_account_id: String,
    pub expense_account_id: String,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    #[serde(default = "default_frequency")]
    pub frequency: String,
}

fn default_frequency() -> String {
    "monthly".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdatePrepaidExpenseSchedule {
    pub is_active: Option<bool>,
    pub description: Option<String>,
}
