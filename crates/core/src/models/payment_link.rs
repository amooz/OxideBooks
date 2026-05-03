use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLink {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: String,
    pub token: String,
    pub amount_due: i64,
    pub currency: String,
    pub status: String,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentLink {
    pub invoice_id: String,
    #[serde(
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub expires_at: Option<OffsetDateTime>,
}
