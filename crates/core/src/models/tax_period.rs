use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxPeriod {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
    pub tax_collected: i64,
    pub tax_paid: i64,
    pub net_tax: i64,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub filed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxPeriod {
    pub name: String,
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
}

#[derive(Debug, Deserialize)]
pub struct FileTaxPeriod {
    pub tax_collected: i64,
    pub tax_paid: i64,
}
