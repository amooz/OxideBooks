use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub id: String,
    pub organization_id: String,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateDepartment {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateDepartment {
    pub code: Option<String>,
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentPlReport {
    pub department_id: String,
    pub department_name: String,
    pub revenue: i64,
    pub expenses: i64,
    pub net: i64,
}
