use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub account_number: Option<String>,
    pub institution: Option<String>,
    pub currency: String,
    pub current_balance: i64,
    pub gl_account_id: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBankAccount {
    pub name: String,
    pub account_number: Option<String>,
    pub institution: Option<String>,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub gl_account_id: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateBankAccount {
    pub name: Option<String>,
    pub account_number: Option<String>,
    pub institution: Option<String>,
    pub gl_account_id: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransaction {
    pub id: String,
    pub bank_account_id: String,
    pub organization_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub txn_date: Date,
    pub description: String,
    pub amount: i64,
    pub txn_type: String,
    pub status: String,
    pub reference: Option<String>,
    pub matched_payment_id: Option<String>,
    pub matched_expense_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBankTransaction {
    #[serde(with = "crate::models::date_serde")]
    pub txn_date: Date,
    pub description: String,
    pub amount: i64,
    pub txn_type: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchTransaction {
    pub payment_id: Option<String>,
    pub expense_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    pub bank_account_id: String,
    pub unmatched_count: i64,
    pub matched_count: i64,
    pub excluded_count: i64,
    pub unmatched_total: i64,
}
