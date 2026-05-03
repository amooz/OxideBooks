use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizedFxEntry {
    pub id: String,
    pub organization_id: String,
    pub payment_id: String,
    pub invoice_currency: String,
    pub payment_currency: String,
    pub invoice_amount: i64,
    pub payment_amount: i64,
    pub fx_rate: f64,
    pub gain_loss: i64,
    pub journal_entry_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxSummaryRow {
    pub period: String,
    pub total_gains: i64,
    pub total_losses: i64,
    pub net: i64,
}
