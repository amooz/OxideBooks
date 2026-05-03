use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredCharge {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub account_id: Option<String>,
    pub description: String,
    #[serde(with = "date_serde")]
    pub charge_date: Date,
    pub amount: MinorUnits,
    pub tax_rate: MinorUnits,
    pub status: String,
    pub invoice_id: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub invoiced_at: Option<OffsetDateTime>,
    pub memo: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeferredCharge {
    pub contact_id: String,
    pub account_id: Option<String>,
    pub description: String,
    #[serde(with = "date_serde")]
    pub charge_date: Date,
    pub amount: MinorUnits,
    #[serde(default)]
    pub tax_rate: MinorUnits,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeferredCharge {
    pub description: Option<String>,
    pub amount: Option<MinorUnits>,
    pub tax_rate: Option<MinorUnits>,
    pub memo: Option<String>,
}

/// Body for POST /deferred-charges/:id/invoice
/// Converts one or more deferred charges into a single invoice.
#[derive(Debug, Deserialize)]
pub struct InvoiceDeferredCharges {
    /// Additional charge IDs to include with this one (same contact required).
    #[serde(default)]
    pub additional_ids: Vec<String>,
    #[serde(with = "date_serde")]
    pub invoice_date: Date,
    #[serde(with = "date_serde")]
    pub due_date: Date,
}

/// Model for progress invoicing: bill a % of an accepted quote.
#[derive(Debug, Deserialize)]
pub struct ProgressInvoiceInput {
    /// Percentage to bill, expressed as integer × 100 (e.g. 5000 = 50%).
    pub pct_bps: i64,
    #[serde(with = "date_serde")]
    pub invoice_date: Date,
    #[serde(with = "date_serde")]
    pub due_date: Date,
}
