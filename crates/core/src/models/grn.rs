use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrnLine {
    pub id: String,
    pub grn_id: String,
    pub po_line_id: String,
    pub item_id: Option<String>,
    pub lot_id: Option<String>,
    pub quantity_received: i64,
    pub unit_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceiptNote {
    pub id: String,
    pub organization_id: String,
    pub purchase_order_id: String,
    #[serde(with = "date_serde")]
    pub receipt_date: Date,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_by: String,
    pub lines: Vec<GrnLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGrnLine {
    pub po_line_id: String,
    pub item_id: Option<String>,
    pub lot_id: Option<String>,
    pub quantity_received: i64,
    #[serde(default)]
    pub unit_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGrn {
    pub purchase_order_id: String,
    #[serde(with = "date_serde")]
    pub receipt_date: Date,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<CreateGrnLine>,
}
