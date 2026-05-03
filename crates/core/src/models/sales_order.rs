use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::{date_serde, opt_date_serde};
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoLine {
    pub id: String,
    pub so_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    pub tax_rate: MinorUnits,
    pub discount_pct: MinorUnits,
    pub quantity_invoiced: i64,
    pub sort_order: i32,
    pub line_total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub id: String,
    pub organization_id: String,
    pub order_number: String,
    pub contact_id: String,
    pub status: String,
    #[serde(with = "date_serde")]
    pub order_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub expected_ship: Option<Date>,
    pub currency: String,
    pub notes: Option<String>,
    pub total_amount: MinorUnits,
    pub invoiced_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    pub lines: Vec<SoLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateSoLine {
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    #[serde(default)]
    pub tax_rate: MinorUnits,
    #[serde(default)]
    pub discount_pct: MinorUnits,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateSalesOrder {
    pub contact_id: String,
    #[serde(with = "date_serde")]
    pub order_date: Date,
    #[serde(default, with = "opt_date_serde")]
    pub expected_ship: Option<Date>,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub lines: Vec<CreateSoLine>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateSalesOrder {
    #[serde(default, with = "opt_date_serde")]
    pub expected_ship: Option<Date>,
    pub currency: Option<String>,
    pub notes: Option<String>,
}

/// Body for POST /sales-orders/:id/convert-to-invoice
#[derive(Debug, Deserialize)]
pub struct ConvertSoToInvoice {
    /// Optional: only invoice specific lines (by line id). Empty = all uninvoiced lines.
    #[serde(default)]
    pub line_ids: Vec<String>,
}
