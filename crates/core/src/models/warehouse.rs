use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseStock {
    pub item_id: String,
    pub product_name: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub code: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateWarehouse {
    pub name: String,
    pub code: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWarehouse {
    pub name: Option<String>,
    pub code: Option<String>,
    pub address: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TransferStock {
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    pub item_id: String,
    pub quantity: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransfer {
    pub id: String,
    pub organization_id: String,
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    pub item_id: String,
    pub quantity: i64,
    pub notes: Option<String>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub transferred_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePendingTransfer {
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    pub item_id: String,
    pub quantity: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockAdjustment {
    pub id: String,
    pub organization_id: String,
    pub warehouse_id: String,
    pub item_id: String,
    pub quantity_delta: i64,
    pub reason: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateStockAdjustment {
    pub item_id: String,
    pub quantity_delta: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSummaryRow {
    pub item_id: String,
    pub product_name: String,
    pub total_quantity: i64,
    pub by_warehouse: Vec<WarehouseStockLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseStockLine {
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub quantity: i64,
}
