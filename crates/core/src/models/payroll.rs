use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollRun {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
    pub status: String,
    pub journal_entry_id: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollEntry {
    pub id: String,
    pub payroll_run_id: String,
    pub user_id: String,
    pub gross_pay: i64,
    pub tax_withheld: i64,
    pub other_deductions: i64,
    pub net_pay: i64,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayrollRun {
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayrollEntry {
    pub user_id: String,
    pub gross_pay: i64,
    #[serde(default)]
    pub tax_withheld: i64,
    #[serde(default)]
    pub other_deductions: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollRunSummary {
    pub run: PayrollRun,
    pub entries: Vec<PayrollEntry>,
    pub total_gross: i64,
    pub total_net: i64,
    pub total_tax: i64,
}
