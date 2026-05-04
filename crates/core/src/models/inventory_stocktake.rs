use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Serialize)]
pub struct InventoryStocktakeLine {
    pub id: String,
    pub stocktake_id: String,
    pub product_id: String,
    pub system_qty: i64,
    pub counted_qty: i64,
    pub variance: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InventoryStocktake {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "date_serde")]
    pub stocktake_date: Date,
    pub warehouse_id: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub lines: Vec<InventoryStocktakeLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateInventoryStocktake {
    #[serde(with = "date_serde")]
    pub stocktake_date: Date,
    pub warehouse_id: Option<String>,
    pub notes: Option<String>,
    /// Products to include; if empty, all active inventory items are included.
    #[serde(default)]
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStocktakeLine {
    pub counted_qty: i64,
    pub notes: Option<String>,
}
