use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSettings {
    pub organization_id: String,
    pub smtp_host: String,
    pub smtp_port: i32,
    pub smtp_user: String,
    pub from_address: String,
    pub from_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertEmailSettings {
    pub smtp_host: String,
    #[serde(default = "default_port")]
    pub smtp_port: i32,
    pub smtp_user: String,
    pub smtp_password: String,
    pub from_address: String,
    #[serde(default = "default_from_name")]
    pub from_name: String,
}

fn default_port() -> i32 {
    587
}

fn default_from_name() -> String {
    "OxideBooks".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLog {
    pub id: String,
    pub organization_id: String,
    pub to_address: String,
    pub subject: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailRequest {
    pub to: String,
    pub subject: Option<String>,
    pub message: Option<String>,
}
