use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub organization_id: String,
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub is_read: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotification {
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}
