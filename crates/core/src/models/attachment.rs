use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub organization_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub file_name: String,
    pub file_size: i64,
    pub content_type: String,
    pub storage_url: String,
    pub uploaded_by: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttachment {
    pub file_name: String,
    #[serde(default)]
    pub file_size: i64,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    pub storage_url: String,
}

fn default_content_type() -> String {
    "application/octet-stream".to_string()
}
