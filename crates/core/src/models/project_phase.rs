use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct ProjectPhase {
    pub id: String,
    pub organization_id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub budget: i64,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub start_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectPhase {
    pub name: String,
    pub description: Option<String>,
    pub budget: Option<i64>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub start_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectPhase {
    pub name: Option<String>,
    pub description: Option<String>,
    pub budget: Option<i64>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub start_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}
