use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceTemplate {
    pub id: String,
    pub organization_id: String,
    pub logo_url: Option<String>,
    pub accent_color: Option<String>,
    pub footer_text: Option<String>,
    pub default_payment_terms_days: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpsertInvoiceTemplate {
    pub logo_url: Option<String>,
    pub accent_color: Option<String>,
    pub footer_text: Option<String>,
    #[serde(default = "default_terms")]
    pub default_payment_terms_days: i32,
}

fn default_terms() -> i32 {
    30
}
