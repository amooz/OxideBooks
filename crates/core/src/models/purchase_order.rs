use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoStatus {
    Draft,
    Sent,
    PartiallyReceived,
    Received,
    Billed,
    Voided,
}

impl std::fmt::Display for PoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PoStatus::Draft => "draft",
            PoStatus::Sent => "sent",
            PoStatus::PartiallyReceived => "partially_received",
            PoStatus::Received => "received",
            PoStatus::Billed => "billed",
            PoStatus::Voided => "voided",
        })
    }
}

impl std::str::FromStr for PoStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "draft" => Ok(Self::Draft),
            "sent" => Ok(Self::Sent),
            "partially_received" => Ok(Self::PartiallyReceived),
            "received" => Ok(Self::Received),
            "billed" => Ok(Self::Billed),
            "voided" => Ok(Self::Voided),
            _ => Err(()),
        }
    }
}

impl PoStatus {
    pub fn can_transition_to(&self, next: &PoStatus) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Sent)
                | (Self::Draft, Self::Voided)
                | (Self::Sent, Self::PartiallyReceived)
                | (Self::Sent, Self::Received)
                | (Self::Sent, Self::Voided)
                | (Self::PartiallyReceived, Self::Received)
                | (Self::PartiallyReceived, Self::Voided)
                | (Self::Received, Self::Billed)
                | (Self::Received, Self::Voided)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub id: String,
    pub po_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub tax_rate: i64,
    pub quantity_received: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: String,
    pub organization_id: String,
    pub po_number: String,
    pub contact_id: String,
    pub status: PoStatus,
    #[serde(with = "crate::models::date_serde")]
    pub order_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub expected_date: Option<Date>,
    pub currency: String,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseOrderLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePoLine {
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i64,
    pub unit_price: i64,
    #[serde(default)]
    pub tax_rate: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseOrder {
    pub contact_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub order_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub expected_date: Option<Date>,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub lines: Vec<CreatePoLine>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdatePurchaseOrder {
    pub status: Option<PoStatus>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub expected_date: Option<Date>,
    pub notes: Option<String>,
}

/// Body for POST /purchase-orders/:id/receive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivePoLine {
    pub line_id: String,
    pub quantity_received: i64,
}
