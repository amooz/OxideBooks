use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backorder {
    pub id: String,
    pub organization_id: String,
    pub so_id: String,
    pub so_line_id: String,
    pub product_id: Option<String>,
    pub quantity: i64,
    pub status: String,
    #[serde(default, with = "opt_date_serde")]
    pub expected_date: Option<Date>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub fulfilled_at: Option<OffsetDateTime>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateBackorder {
    pub so_id: String,
    pub so_line_id: String,
    pub product_id: Option<String>,
    pub quantity: i64,
    pub expected_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FulfillBackorder {
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropShipRequest {
    pub id: String,
    pub organization_id: String,
    pub so_id: String,
    pub so_line_id: String,
    pub po_id: Option<String>,
    pub vendor_id: String,
    pub product_id: Option<String>,
    pub quantity: i64,
    pub status: String,
    pub ship_to_name: Option<String>,
    pub ship_to_address: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDropShipRequest {
    pub so_id: String,
    pub so_line_id: String,
    pub vendor_id: String,
    pub product_id: Option<String>,
    pub quantity: i64,
    pub ship_to_name: Option<String>,
    pub ship_to_address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDropShipRequest {
    pub po_id: Option<String>,
    pub status: Option<String>,
    pub ship_to_name: Option<String>,
    pub ship_to_address: Option<String>,
    pub notes: Option<String>,
}
