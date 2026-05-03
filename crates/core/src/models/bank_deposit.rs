use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankDepositItem {
    pub id: String,
    pub deposit_id: String,
    pub payment_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankDeposit {
    pub id: String,
    pub organization_id: String,
    pub bank_account_id: String,
    #[serde(with = "date_serde")]
    pub deposit_date: Date,
    pub currency: String,
    pub total_amount: i64,
    pub reference: Option<String>,
    pub memo: Option<String>,
    pub status: String,
    pub items: Vec<BankDepositItem>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDepositItem {
    pub payment_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateBankDeposit {
    pub bank_account_id: String,
    #[serde(with = "date_serde")]
    pub deposit_date: Date,
    pub currency: Option<String>,
    pub reference: Option<String>,
    pub memo: Option<String>,
    pub items: Vec<CreateDepositItem>,
}

#[derive(Debug, Deserialize)]
pub struct ClearBankDeposit {
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub cleared_date: Option<Date>,
}
