use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseReport {
    pub id: String,
    pub organization_id: String,
    pub title: String,
    pub employee_id: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub total_amount: MinorUnits,
    pub approved_by: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub reimbursed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseReport {
    pub title: String,
    pub employee_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpenseReport {
    pub title: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddExpenseToReport {
    pub expense_id: String,
}
