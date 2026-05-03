use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveType {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub days_per_year: f64,
    pub is_paid: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateLeaveType {
    pub name: String,
    #[serde(default)]
    pub days_per_year: f64,
    #[serde(default = "default_true")]
    pub is_paid: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateLeaveType {
    pub name: Option<String>,
    pub days_per_year: Option<f64>,
    pub is_paid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub id: String,
    pub organization_id: String,
    pub employee_id: String,
    pub leave_type_id: String,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    pub days: f64,
    pub status: String,
    pub notes: Option<String>,
    pub approved_by: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateLeaveRequest {
    pub employee_id: String,
    pub leave_type_id: String,
    #[serde(with = "date_serde")]
    pub start_date: Date,
    #[serde(with = "date_serde")]
    pub end_date: Date,
    pub days: f64,
    pub notes: Option<String>,
}

fn default_true() -> bool {
    true
}
