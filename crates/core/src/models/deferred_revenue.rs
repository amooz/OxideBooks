use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredRevenueEntry {
    pub id: String,
    pub schedule_id: String,
    pub recognition_date: Date,
    pub amount: MinorUnits,
    pub journal_entry_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredRevenueSchedule {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: Option<String>,
    pub invoice_line_id: Option<String>,
    pub deferred_account_id: String,
    pub revenue_account_id: String,
    pub description: String,
    pub total_amount: MinorUnits,
    pub recognized_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    pub frequency: String,
    pub status: String,
    pub entries: Vec<DeferredRevenueEntry>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeferredRevenueSchedule {
    pub invoice_id: Option<String>,
    pub invoice_line_id: Option<String>,
    pub deferred_account_id: String,
    pub revenue_account_id: String,
    pub description: String,
    pub total_amount: MinorUnits,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    #[serde(default = "default_frequency")]
    pub frequency: String,
}

fn default_frequency() -> String {
    "monthly".to_string()
}

/// Body for POST /deferred-revenue/:id/recognize
/// Recognizes one entry (or all pending entries up to today if `entry_id` omitted).
#[derive(Debug, Deserialize)]
pub struct RecognizeRevenue {
    pub entry_id: Option<String>,
}
