use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::{date_serde, opt_date_serde};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeLoan {
    pub id: String,
    pub organization_id: String,
    pub employee_id: String,
    /// Original principal in minor units
    pub amount: i64,
    /// Remaining unpaid balance in minor units
    pub balance: i64,
    pub purpose: Option<String>,
    pub account_id: Option<String>,
    #[serde(with = "date_serde")]
    pub loan_date: Date,
    pub status: String,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmployeeLoan {
    pub employee_id: String,
    pub amount: i64,
    pub purpose: Option<String>,
    pub account_id: Option<String>,
    #[serde(with = "date_serde")]
    pub loan_date: Date,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanRepayment {
    pub id: String,
    pub loan_id: String,
    #[serde(with = "date_serde")]
    pub repayment_date: Date,
    pub amount: i64,
    pub payslip_id: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateLoanRepayment {
    #[serde(with = "date_serde")]
    pub repayment_date: Date,
    pub amount: i64,
    pub payslip_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateEmployeeLoan {
    pub purpose: Option<String>,
    pub account_id: Option<String>,
    pub notes: Option<String>,
    #[serde(default, with = "opt_date_serde")]
    pub loan_date: Option<Date>,
}
