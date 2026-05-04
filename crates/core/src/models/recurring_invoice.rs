use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::{models::opt_date_serde, money::MinorUnits};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringInvoiceLine {
    pub id: String,
    pub recurring_invoice_id: String,
    pub description: String,
    pub quantity: i32,
    pub unit_price: MinorUnits,
    pub account_id: Option<String>,
    pub tax_rate: MinorUnits,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringInvoice {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub description: String,
    pub reference: Option<String>,
    pub currency_code: String,
    pub frequency: String,
    pub interval_count: i32,
    pub next_due_date: Date,
    #[serde(with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub is_active: bool,
    pub days_due: i32,
    pub lines: Vec<RecurringInvoiceLine>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecurringInvoiceLine {
    pub description: String,
    pub quantity: i32,
    pub unit_price: MinorUnits,
    pub account_id: Option<String>,
    #[serde(default)]
    pub tax_rate: MinorUnits,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecurringInvoice {
    pub contact_id: String,
    pub description: String,
    pub reference: Option<String>,
    pub currency_code: Option<String>,
    pub frequency: String,
    #[serde(default = "default_interval")]
    pub interval_count: i32,
    pub next_due_date: Date,
    #[serde(with = "opt_date_serde", default)]
    pub end_date: Option<Date>,
    #[serde(default = "default_days_due")]
    pub days_due: i32,
    pub lines: Vec<CreateRecurringInvoiceLine>,
}

fn default_interval() -> i32 {
    1
}
fn default_days_due() -> i32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecurringInvoice {
    pub description: Option<String>,
    pub reference: Option<String>,
    pub frequency: Option<String>,
    pub interval_count: Option<i32>,
    pub next_due_date: Option<Date>,
    #[serde(with = "opt_date_serde", default)]
    pub end_date: Option<Date>,
    pub days_due: Option<i32>,
    pub is_active: Option<bool>,
}
