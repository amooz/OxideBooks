use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub const ALL_EVENT_TYPES: &[&str] = &[
    "invoice.created",
    "invoice.updated",
    "payment.created",
    "contact.created",
    "contact.deleted",
    "expense.approved",
    "expense.rejected",
    "journal_entry.created",
    "journal_entry.voided",
    "purchase_order.created",
    "purchase_order.updated",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    pub id: String,
    pub organization_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookEndpoint {
    pub url: String,
    pub events: Vec<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateWebhookEndpoint {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

/// Payload sent to a webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub organization_id: String,
    pub data: serde_json::Value,
}
