use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankReconciliationStatement {
    pub id: String,
    pub organization_id: String,
    pub bank_account_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub statement_date: Date,
    pub statement_balance: i64,
    pub book_balance: i64,
    pub outstanding_deposits: i64,
    pub outstanding_checks: i64,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateBankReconciliationStatement {
    pub bank_account_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub statement_date: Date,
    pub statement_balance: i64,
    pub notes: Option<String>,
}
