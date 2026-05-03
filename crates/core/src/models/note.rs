use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub organization_id: String,
    pub user_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub body: String,
    pub is_system: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNote {
    pub body: String,
}
