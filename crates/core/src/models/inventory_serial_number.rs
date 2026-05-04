use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySerialNumber {
    pub id: String,
    pub organization_id: String,
    pub product_id: String,
    pub serial_number: String,
    pub status: String,
    pub lot_id: Option<String>,
    pub warehouse_id: Option<String>,
    #[serde(
        with = "crate::models::opt_date_serde",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub purchase_date: Option<Date>,
    #[serde(
        with = "crate::models::opt_date_serde",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub sold_date: Option<Date>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateInventorySerialNumber {
    pub product_id: String,
    pub serial_number: String,
    pub lot_id: Option<String>,
    pub warehouse_id: Option<String>,
    #[serde(
        with = "crate::models::opt_date_serde",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub purchase_date: Option<Date>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInventorySerialNumber {
    pub status: Option<String>,
    pub lot_id: Option<String>,
    pub warehouse_id: Option<String>,
    #[serde(
        with = "crate::models::opt_date_serde",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub sold_date: Option<Date>,
    pub notes: Option<String>,
}
