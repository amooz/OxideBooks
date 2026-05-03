use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepayment {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub amount: MinorUnits,
    pub reference: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    pub applied_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePrepayment {
    pub contact_id: String,
    pub amount: MinorUnits,
    pub reference: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
}

#[derive(Debug, Deserialize)]
pub struct ApplyPrepayment {
    pub invoice_id: String,
    pub amount: MinorUnits,
}
