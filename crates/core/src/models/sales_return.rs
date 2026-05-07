use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReturnLine {
    pub id: String,
    pub return_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub restock: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReturn {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: Option<String>,
    pub contact_id: Option<String>,
    pub rma_number: String,
    pub status: String,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub credit_note_id: Option<String>,
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
    pub received_at: Option<OffsetDateTime>,
    pub lines: Vec<SalesReturnLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesReturnLine {
    pub product_id: Option<String>,
    pub description: String,
    #[serde(default = "default_qty")]
    pub quantity: i64,
    pub unit_price: i64,
    #[serde(default = "default_true")]
    pub restock: bool,
}

fn default_qty() -> i64 {
    100
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesReturn {
    pub invoice_id: Option<String>,
    pub contact_id: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<CreateSalesReturnLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApproveSalesReturn {
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReceiveSalesReturn {
    pub notes: Option<String>,
}
