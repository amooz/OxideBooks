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
    pub created_at: OffsetDateTime,
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
