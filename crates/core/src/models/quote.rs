use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::{date_serde, opt_date_serde};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteLine {
    pub id: String,
    pub quote_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub discount_pct: i64,
    pub tax_rate: i64,
    pub sort_order: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub organization_id: String,
    pub contact_id: Option<String>,
    pub quote_number: String,
    pub status: String,
    #[serde(with = "date_serde")]
    pub issue_date: Date,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_date_serde"
    )]
    pub expiry_date: Option<Date>,
    pub currency: String,
    pub exchange_rate: Decimal,
    pub notes: Option<String>,
    pub terms: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub sent_at: Option<OffsetDateTime>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub accepted_at: Option<OffsetDateTime>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub declined_at: Option<OffsetDateTime>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub invoiced_at: Option<OffsetDateTime>,
    pub converted_invoice_id: Option<String>,
    pub lines: Vec<QuoteLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuoteLine {
    pub product_id: Option<String>,
    pub description: String,
    #[serde(default = "default_qty")]
    pub quantity: i64,
    pub unit_price: i64,
    #[serde(default)]
    pub discount_pct: i64,
    #[serde(default)]
    pub tax_rate: i64,
    #[serde(default)]
    pub sort_order: i32,
}

fn default_qty() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuote {
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub issue_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub expiry_date: Option<Date>,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub lines: Vec<CreateQuoteLine>,
}

fn default_currency() -> String {
    "USD".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateQuote {
    pub contact_id: Option<String>,
    #[serde(default, with = "opt_date_serde")]
    pub issue_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub expiry_date: Option<Date>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub lines: Option<Vec<CreateQuoteLine>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertQuoteToInvoice {
    #[serde(default, with = "opt_date_serde")]
    pub invoice_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
}
