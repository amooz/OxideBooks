use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingOption {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub is_active: bool,
    pub sort_order: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingCategory {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub is_active: bool,
    pub options: Vec<TrackingOption>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateTrackingCategory {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTrackingCategory {
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTrackingOption {
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTrackingOption {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}
