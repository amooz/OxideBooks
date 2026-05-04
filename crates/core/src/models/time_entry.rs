use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    pub project_id: Option<String>,
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub entry_date: Date,
    pub minutes: i32,
    pub description: String,
    pub hourly_rate: i64,
    pub is_billable: bool,
    pub invoice_line_id: Option<String>,
    pub approval_status: String,
    pub approved_by: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub approved_at: Option<OffsetDateTime>,
    pub rejection_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTimeEntry {
    pub project_id: Option<String>,
    pub contact_id: Option<String>,
    #[serde(with = "date_serde")]
    pub entry_date: Date,
    pub minutes: i32,
    pub description: String,
    #[serde(default)]
    pub hourly_rate: i64,
    #[serde(default = "default_true")]
    pub is_billable: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTimeEntry {
    pub project_id: Option<String>,
    pub minutes: Option<i32>,
    pub description: Option<String>,
    pub hourly_rate: Option<i64>,
    pub is_billable: Option<bool>,
}

/// Bill selected time entries — attaches them to an existing invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillTimeEntries {
    pub invoice_id: String,
    pub entry_ids: Vec<String>,
    /// GL account for the new invoice lines (revenue account)
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSummaryRow {
    pub user_id: String,
    pub project_id: Option<String>,
    pub total_minutes: i64,
    pub billable_minutes: i64,
    pub billable_amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RejectTimeEntry {
    pub reason: Option<String>,
}
