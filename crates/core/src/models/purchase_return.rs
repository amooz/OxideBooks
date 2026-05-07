use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturnLine {
    pub id: String,
    pub return_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturn {
    pub id: String,
    pub organization_id: String,
    pub bill_id: Option<String>,
    pub contact_id: Option<String>,
    pub rma_number: String,
    pub status: String,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub vendor_credit_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub shipped_at: Option<OffsetDateTime>,
    pub lines: Vec<PurchaseReturnLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseReturnLine {
    pub product_id: Option<String>,
    pub description: String,
    #[serde(default = "default_qty")]
    pub quantity: i64,
    pub unit_price: i64,
}

fn default_qty() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseReturn {
    pub bill_id: Option<String>,
    pub contact_id: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<CreatePurchaseReturnLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApprovePurchaseReturn {
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShipPurchaseReturn {
    pub notes: Option<String>,
}
