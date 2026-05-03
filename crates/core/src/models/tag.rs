use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub color: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#6366f1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTag {
    pub name: Option<String>,
    pub color: Option<String>,
}
