use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLot {
    pub id: String,
    pub organization_id: String,
    pub item_id: String,
    pub lot_number: String,
    #[serde(default, with = "opt_date_serde")]
    pub expiry_date: Option<Date>,
    pub quantity: i64,
    pub cost_per_unit: i64,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInventoryLot {
    pub item_id: String,
    pub lot_number: String,
    #[serde(default, with = "opt_date_serde")]
    pub expiry_date: Option<Date>,
    pub quantity: i64,
    pub cost_per_unit: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateInventoryLot {
    #[serde(default, with = "opt_date_serde")]
    pub expiry_date: Option<Date>,
    pub quantity: Option<i64>,
    pub cost_per_unit: Option<i64>,
    pub notes: Option<String>,
}
