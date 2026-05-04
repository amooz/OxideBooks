use rust_decimal::Decimal;
use serde::Deserialize;
use time::Date;

use crate::models::{date_serde, CreateInvoiceLine};

#[derive(Debug, Deserialize)]
pub struct CreateCashSale {
    pub contact_id: String,
    #[serde(with = "date_serde")]
    pub date: Date,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub notes: Option<String>,
    /// GL account (or bank account) into which the payment lands
    pub payment_account_id: String,
    #[serde(default = "default_method")]
    pub payment_method: String,
    pub lines: Vec<CreateInvoiceLine>,
}

fn default_method() -> String {
    "cash".into()
}
