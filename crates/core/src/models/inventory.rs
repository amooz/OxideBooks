use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub organization_id: String,
    pub product_id: String,
    pub quantity_on_hand: i64,
    pub reorder_point: i64,
    pub reorder_qty: i64,
    pub cost_per_unit: i64,
    pub valuation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInventoryItem {
    pub product_id: String,
    #[serde(default)]
    pub quantity_on_hand: i64,
    #[serde(default)]
    pub reorder_point: i64,
    #[serde(default)]
    pub reorder_qty: i64,
    #[serde(default)]
    pub cost_per_unit: i64,
    #[serde(default = "default_valuation")]
    pub valuation_method: String,
}

fn default_valuation() -> String {
    "average".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInventoryItem {
    pub reorder_point: Option<i64>,
    pub reorder_qty: Option<i64>,
    pub cost_per_unit: Option<i64>,
    pub valuation_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryAdjustment {
    pub quantity: i64,
    pub unit_cost: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryMovement {
    pub id: String,
    pub organization_id: String,
    pub item_id: String,
    pub movement_type: String,
    pub quantity: i64,
    pub unit_cost: i64,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity_on_hand: i64,
    pub reorder_point: i64,
    pub reorder_qty: i64,
    pub shortfall: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryValuationRow {
    pub product_id: String,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity_on_hand: i64,
    pub cost_per_unit: i64,
    pub total_value: i64,
    pub valuation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryValuationReport {
    pub rows: Vec<InventoryValuationRow>,
    pub total_value: i64,
}
