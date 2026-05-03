use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPayment {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub payment_date: Date,
    pub method: String,
    pub reference: Option<String>,
    pub total_amount: i64,
    pub created_by: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub lines: Vec<BatchPaymentLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPaymentLine {
    pub id: String,
    pub batch_payment_id: String,
    pub invoice_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateBatchPayment {
    pub invoice_ids: Vec<String>,
    /// ISO date string YYYY-MM-DD; defaults to today if absent
    pub payment_date: Option<String>,
    pub method: String,
    pub reference: Option<String>,
}
