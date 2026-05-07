use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub contact_id: Option<String>,
    pub status: String,
    pub billing_method: String,
    pub budget_amount: Option<i64>,
    #[serde(default, with = "opt_date_serde")]
    pub start_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub contact_id: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_billing_method")]
    pub billing_method: String,
    pub budget_amount: Option<i64>,
    #[serde(default, with = "opt_date_serde")]
    pub start_date: Option<Date>,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub notes: Option<String>,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_billing_method() -> String {
    "time_and_materials".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub status: Option<String>,
    pub billing_method: Option<String>,
    pub budget_amount: Option<i64>,
    #[serde(default, with = "opt_date_serde")]
    pub end_date: Option<Date>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub project_name: String,
    pub total_invoiced: i64,
    pub total_expenses: i64,
    pub total_time_cost: i64,
    pub net: i64,
}
