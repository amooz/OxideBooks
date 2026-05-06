use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::opt_date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTask {
    pub id: String,
    pub organization_id: String,
    pub project_id: String,
    pub phase_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub assignee_id: Option<String>,
    pub status: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_date_serde"
    )]
    pub due_date: Option<Date>,
    pub estimated_minutes: Option<i32>,
    pub actual_minutes: i32,
    pub sort_order: i32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectTask {
    pub name: String,
    pub description: Option<String>,
    pub phase_id: Option<String>,
    pub assignee_id: Option<String>,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub estimated_minutes: Option<i32>,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProjectTask {
    pub name: Option<String>,
    pub description: Option<String>,
    pub phase_id: Option<String>,
    pub assignee_id: Option<String>,
    pub status: Option<String>,
    #[serde(default, with = "opt_date_serde")]
    pub due_date: Option<Date>,
    pub estimated_minutes: Option<i32>,
    pub actual_minutes: Option<i32>,
    pub sort_order: Option<i32>,
}
