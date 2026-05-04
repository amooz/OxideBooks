use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: String,
    pub amount: i64,
    pub payment_date: Date,
    pub method: String,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub realized_fx_amount: i64,
    pub fx_journal_entry_id: Option<String>,
    pub voided_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub id: String,
    pub payment_id: String,
    pub amount: i64,
    pub reason: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub refund_date: Date,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateRefund {
    pub amount: i64,
    pub reason: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub refund_date: Date,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayment {
    pub amount: i64,
    #[serde(with = "crate::models::date_serde")]
    pub payment_date: Date,
    #[serde(default = "default_method")]
    pub method: String,
    pub reference: Option<String>,
    pub notes: Option<String>,
    /// Exchange rate at time of payment (base-currency units per 1 foreign unit, scaled ×10000).
    /// Provide when the invoice currency differs from the org base currency to trigger
    /// realized FX gain/loss calculation.
    pub exchange_rate: Option<rust_decimal::Decimal>,
}

fn default_method() -> String {
    "bank_transfer".into()
}

pub const VALID_METHODS: &[&str] = &[
    "bank_transfer",
    "cash",
    "check",
    "credit_card",
    "direct_debit",
    "other",
];
