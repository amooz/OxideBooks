use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesCommission {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: String,
    pub salesperson_id: String,
    /// Rate in basis points (1% = 100 bps)
    pub rate_bps: i32,
    /// Amount in minor units
    pub amount: i64,
    pub status: String,
    #[serde(default, with = "opt_date_serde")]
    pub payment_date: Option<Date>,
    pub payment_ref: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateSalesCommission {
    pub invoice_id: String,
    pub salesperson_id: String,
    /// Rate in basis points (1% = 100 bps)
    pub rate_bps: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PayCommission {
    #[serde(with = "crate::models::date_serde")]
    pub payment_date: Date,
    pub payment_ref: Option<String>,
}
