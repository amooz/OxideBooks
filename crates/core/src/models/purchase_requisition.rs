use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrLine {
    pub id: String,
    pub requisition_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    pub account_id: Option<String>,
    pub sort_order: i32,
    pub line_total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRequisition {
    pub id: String,
    pub organization_id: String,
    pub requester_id: Option<String>,
    pub approver_id: Option<String>,
    pub title: String,
    pub notes: Option<String>,
    pub status: String,
    pub total_amount: MinorUnits,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub rejected_at: Option<OffsetDateTime>,
    pub converted_po_id: Option<String>,
    pub lines: Vec<PrLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePrLine {
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: MinorUnits,
    pub account_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseRequisition {
    pub title: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub lines: Vec<CreatePrLine>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePurchaseRequisition {
    pub title: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertPrToPo {
    pub vendor_id: String,
}
