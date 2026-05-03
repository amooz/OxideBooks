use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorCreditLine {
    pub id: String,
    pub credit_id: String,
    pub account_id: Option<String>,
    pub description: Option<String>,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    pub tax_rate: MinorUnits,
    pub sort_order: i32,
    pub line_total: MinorUnits,
}

impl VendorCreditLine {
    pub fn line_total(&self) -> MinorUnits {
        self.quantity * self.unit_price / 100
            + self.quantity * self.unit_price / 100 * self.tax_rate / 10_000
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorCredit {
    pub id: String,
    pub organization_id: String,
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub credit_date: Date,
    pub reference: Option<String>,
    pub memo: Option<String>,
    pub status: String,
    pub total_amount: MinorUnits,
    pub applied_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    pub lines: Vec<VendorCreditLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorCreditApplication {
    pub id: String,
    pub organization_id: String,
    pub credit_id: String,
    pub bill_id: String,
    pub amount: MinorUnits,
    #[serde(with = "time::serde::rfc3339")]
    pub applied_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateVendorCreditLine {
    pub account_id: Option<String>,
    pub description: Option<String>,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    #[serde(default)]
    pub tax_rate: MinorUnits,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateVendorCredit {
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub credit_date: Date,
    pub reference: Option<String>,
    pub memo: Option<String>,
    #[serde(default)]
    pub lines: Vec<CreateVendorCreditLine>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyVendorCredit {
    pub bill_id: String,
    pub amount: MinorUnits,
}
