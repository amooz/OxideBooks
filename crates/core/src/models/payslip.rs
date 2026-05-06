use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payslip {
    pub id: String,
    pub organization_id: String,
    pub payroll_run_id: String,
    pub employee_id: String,
    pub gross_pay: MinorUnits,
    pub tax_withheld: MinorUnits,
    pub deductions: MinorUnits,
    pub net_pay: MinorUnits,
    pub notes: Option<String>,
    pub status: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreatePayslip {
    pub employee_id: String,
    pub gross_pay: MinorUnits,
    #[serde(default)]
    pub tax_withheld: MinorUnits,
    #[serde(default)]
    pub deductions: MinorUnits,
    pub notes: Option<String>,
}
