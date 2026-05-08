use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub lease_type: String,
    pub asset_account_id: Option<String>,
    pub liability_account_id: Option<String>,
    pub expense_account_id: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub commencement_date: Date,
    #[serde(with = "crate::models::date_serde")]
    pub end_date: Date,
    pub payment_amount: i64,
    pub payment_frequency: String,
    pub discount_rate_bps: i32,
    pub initial_rou_asset: i64,
    pub initial_liability: i64,
    pub status: String,
    #[serde(default, with = "opt_date_serde")]
    pub terminated_at: Option<Date>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateLease {
    pub name: String,
    pub description: Option<String>,
    pub lease_type: String,
    pub asset_account_id: Option<String>,
    pub liability_account_id: Option<String>,
    pub expense_account_id: Option<String>,
    pub commencement_date: String,
    pub end_date: String,
    pub payment_amount: i64,
    #[serde(default = "default_frequency")]
    pub payment_frequency: String,
    pub discount_rate_bps: i32,
}

fn default_frequency() -> String {
    "monthly".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseScheduleLine {
    pub period: i32,
    #[serde(with = "crate::models::date_serde")]
    pub period_date: Date,
    pub payment: i64,
    pub interest: i64,
    pub principal: i64,
    pub rou_amortization: i64,
    pub lease_expense: i64,
    pub liability_balance: i64,
    pub rou_balance: i64,
    pub is_posted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeasePayment {
    pub id: String,
    pub lease_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub period_date: Date,
    pub payment: i64,
    pub interest: i64,
    pub principal: i64,
    pub rou_amort: i64,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct RecordLeasePayment {
    pub period_date: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TerminateLease {
    pub terminated_at: String,
}
