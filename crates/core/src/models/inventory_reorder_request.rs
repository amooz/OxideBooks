use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize)]
pub struct InventoryReorderRequest {
    pub id: String,
    pub organization_id: String,
    pub product_id: String,
    pub supplier_id: Option<String>,
    pub requested_qty: i64,
    pub status: String,
    pub purchase_order_id: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateInventoryReorderRequest {
    pub product_id: String,
    pub supplier_id: Option<String>,
    /// If omitted the repo falls back to the item's `reorder_qty`.
    pub requested_qty: Option<i64>,
    pub notes: Option<String>,
}

/// Submit the reorder request — creates a draft purchase order.
#[derive(Debug, Deserialize)]
pub struct SubmitInventoryReorderRequest {
    /// Preferred delivery date for the resulting PO.
    pub delivery_date: Option<String>,
}
