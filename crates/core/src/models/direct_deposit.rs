use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDepositEntry {
    pub id: String,
    pub batch_id: String,
    pub employee_id: String,
    pub employee_bank_id: Option<String>,
    pub amount: MinorUnits,
    pub routing_number: Option<String>,
    pub account_number: Option<String>,
    pub account_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDepositBatch {
    pub id: String,
    pub organization_id: String,
    pub payroll_run_id: Option<String>,
    pub bank_account_id: Option<String>,
    pub batch_date: Date,
    pub status: String,
    pub total_amount: MinorUnits,
    pub entry_count: i32,
    pub reference: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub sent_at: Option<OffsetDateTime>,
    pub entries: Vec<DirectDepositEntry>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDirectDepositEntry {
    pub employee_id: String,
    pub employee_bank_id: Option<String>,
    pub amount: MinorUnits,
    pub routing_number: Option<String>,
    pub account_number: Option<String>,
    #[serde(default = "default_account_type")]
    pub account_type: String,
}

fn default_account_type() -> String {
    "checking".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDirectDepositBatch {
    pub payroll_run_id: Option<String>,
    pub bank_account_id: Option<String>,
    pub batch_date: Date,
    pub reference: Option<String>,
    pub entries: Vec<CreateDirectDepositEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkBatchSent {
    pub reference: Option<String>,
}
