use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldDefinition {
    pub id: String,
    pub organization_id: String,
    pub entity_type: String,
    pub name: String,
    pub field_type: String,
    pub is_required: bool,
    pub sort_order: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCustomFieldDefinition {
    pub entity_type: String,
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateCustomFieldDefinition {
    pub name: Option<String>,
    pub is_required: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldValue {
    pub definition_id: String,
    pub entity_id: String,
    pub name: String,
    pub field_type: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCustomFieldValue {
    pub definition_id: String,
    pub value: Option<String>,
}
