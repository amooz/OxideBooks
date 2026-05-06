use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderShipment {
    pub id: String,
    pub organization_id: String,
    pub sales_order_id: String,
    #[serde(with = "date_serde")]
    pub shipped_at: Date,
    pub tracking_number: Option<String>,
    pub carrier: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<ShipmentLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentLine {
    pub id: String,
    pub shipment_id: String,
    pub so_line_id: String,
    pub product_id: Option<String>,
    pub quantity_shipped: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesOrderShipment {
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub shipped_at: Option<Date>,
    pub tracking_number: Option<String>,
    pub carrier: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<CreateShipmentLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShipmentLine {
    pub so_line_id: String,
    pub quantity_shipped: i64,
}
