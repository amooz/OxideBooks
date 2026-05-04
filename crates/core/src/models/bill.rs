use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::{date_serde, opt_date_serde};
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorBill {
    pub id: String,
    pub organization_id: String,
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub bill_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub reference: Option<String>,
    pub description: String,
    pub status: String,
    pub doc_number: Option<String>,
    pub currency_code: String,
    pub exchange_rate: rust_decimal::Decimal,
    pub lines: Vec<BillLine>,
    pub total: MinorUnits,
    pub amount_paid: MinorUnits,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillLine {
    pub id: String,
    pub bill_id: String,
    pub account_id: Option<String>,
    pub description: Option<String>,
    pub quantity: i32,
    pub unit_price: MinorUnits,
    pub tax_rate: MinorUnits,
}

impl BillLine {
    pub fn line_total(&self) -> MinorUnits {
        self.quantity as i64 * self.unit_price
            + self.quantity as i64 * self.unit_price * self.tax_rate / 1_000_000
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillPayment {
    pub id: String,
    pub organization_id: String,
    pub bill_id: String,
    #[serde(with = "date_serde")]
    pub payment_date: Date,
    pub amount: MinorUnits,
    pub method: String,
    pub reference: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateVendorBill {
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub bill_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub reference: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_currency")]
    pub currency_code: String,
    #[serde(default = "default_rate")]
    pub exchange_rate: rust_decimal::Decimal,
    pub lines: Vec<CreateBillLine>,
    pub purchase_order_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBillLine {
    pub account_id: Option<String>,
    pub description: Option<String>,
    pub quantity: i32,
    pub unit_price: MinorUnits,
    #[serde(default)]
    pub tax_rate: MinorUnits,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateVendorBill {
    pub contact_id: Option<String>,
    #[serde(default, with = "opt_date_serde")]
    pub bill_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub reference: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBillPayment {
    #[serde(with = "date_serde")]
    pub payment_date: Date,
    pub amount: MinorUnits,
    #[serde(default = "default_method")]
    pub method: String,
    pub reference: Option<String>,
}

/// Input for POST /bills/spend-money — creates a vendor bill + immediate payment atomically.
#[derive(Debug, Deserialize)]
pub struct CreateSpendMoney {
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub date: Date,
    pub payment_account_id: String,
    #[serde(default = "default_method")]
    pub payment_method: String,
    pub reference: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_currency")]
    pub currency_code: String,
    #[serde(default = "default_rate")]
    pub exchange_rate: rust_decimal::Decimal,
    pub lines: Vec<CreateBillLine>,
}

fn default_method() -> String {
    "bank_transfer".into()
}

fn default_currency() -> String {
    "USD".into()
}

fn default_rate() -> rust_decimal::Decimal {
    rust_decimal::Decimal::ONE
}
