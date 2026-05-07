use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;
use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevRecEntry {
    pub id: String,
    pub schedule_id: String,
    pub organization_id: String,
    #[serde(with = "date_serde")]
    pub period: Date,
    pub amount: MinorUnits,
    pub journal_entry_id: Option<String>,
    pub posted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevRecSchedule {
    pub id: String,
    pub organization_id: String,
    pub invoice_id: Option<String>,
    pub revenue_account_id: Option<String>,
    pub deferred_account_id: Option<String>,
    pub description: String,
    pub method: String,
    pub total_amount: MinorUnits,
    pub recognized_amount: MinorUnits,
    pub remaining_amount: MinorUnits,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    pub status: String,
    pub entries: Vec<RevRecEntry>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRevRecSchedule {
    pub revenue_account_id: Option<String>,
    pub deferred_account_id: Option<String>,
    pub description: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub total_amount: MinorUnits,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
}

/// Body for POST /rev-rec/recognize
#[derive(Debug, Clone, Deserialize)]
pub struct RecognizeRevRec {
    /// Period in YYYY-MM format (recognizes all schedules with entries in that month)
    pub period: String,
    /// If provided, only recognize this specific schedule
    pub schedule_id: Option<String>,
}

fn default_method() -> String {
    "straight_line".to_string()
}
