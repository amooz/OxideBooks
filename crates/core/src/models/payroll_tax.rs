use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollTaxLiability {
    pub id: String,
    pub organization_id: String,
    pub payroll_run_id: String,
    pub tax_type: String,
    pub employee_amount: i64,
    pub employer_amount: i64,
    #[serde(with = "crate::models::date_serde")]
    pub period_start: Date,
    #[serde(with = "crate::models::date_serde")]
    pub period_end: Date,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub paid_date: Option<Date>,
    pub status: String,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePayrollTaxLiability {
    pub payroll_run_id: String,
    pub tax_type: String,
    pub employee_amount: i64,
    pub employer_amount: i64,
    #[serde(with = "crate::models::date_serde")]
    pub period_start: Date,
    #[serde(with = "crate::models::date_serde")]
    pub period_end: Date,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PayPayrollTax {
    #[serde(with = "crate::models::date_serde")]
    pub paid_date: Date,
    pub notes: Option<String>,
}
