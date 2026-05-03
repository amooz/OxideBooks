use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseStatus {
    Draft,
    Submitted,
    Approved,
    Rejected,
    Reimbursed,
}

impl std::fmt::Display for ExpenseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ExpenseStatus::Draft => "draft",
            ExpenseStatus::Submitted => "submitted",
            ExpenseStatus::Approved => "approved",
            ExpenseStatus::Rejected => "rejected",
            ExpenseStatus::Reimbursed => "reimbursed",
        })
    }
}

impl std::str::FromStr for ExpenseStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "draft" => Ok(Self::Draft),
            "submitted" => Ok(Self::Submitted),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "reimbursed" => Ok(Self::Reimbursed),
            _ => Err(()),
        }
    }
}

impl ExpenseStatus {
    pub fn can_transition_to(&self, next: &ExpenseStatus) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Submitted)
                | (Self::Submitted, Self::Approved)
                | (Self::Submitted, Self::Rejected)
                | (Self::Approved, Self::Reimbursed)
                | (Self::Approved, Self::Rejected)
                | (Self::Rejected, Self::Draft)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub expense_date: Date,
    pub amount: i64,
    pub currency: String,
    pub category: String,
    pub description: String,
    pub account_id: Option<String>,
    pub status: ExpenseStatus,
    pub receipt_url: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExpense {
    #[serde(with = "crate::models::date_serde")]
    pub expense_date: Date,
    pub amount: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub category: String,
    pub description: String,
    pub account_id: Option<String>,
    pub receipt_url: Option<String>,
    pub notes: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateExpense {
    #[serde(default, with = "opt_date_serde")]
    pub expense_date: Option<Date>,
    pub amount: Option<i64>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub account_id: Option<String>,
    pub receipt_url: Option<String>,
    pub notes: Option<String>,
}
