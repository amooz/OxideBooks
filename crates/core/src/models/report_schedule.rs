use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub report_type: String,
    pub frequency: String,
    pub params: serde_json::Value,
    pub recipients: Vec<String>,
    pub is_active: bool,
    pub last_run_at: Option<OffsetDateTime>,
    pub next_run_at: Option<OffsetDateTime>,
    pub created_by: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportSchedule {
    pub name: String,
    pub report_type: String,
    pub frequency: String,
    #[serde(default = "default_params")]
    pub params: serde_json::Value,
    #[serde(default)]
    pub recipients: Vec<String>,
}

fn default_params() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReportSchedule {
    pub name: Option<String>,
    pub frequency: Option<String>,
    pub params: Option<serde_json::Value>,
    pub recipients: Option<Vec<String>>,
    pub is_active: Option<bool>,
}
