use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCategory {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductCategory {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateProductCategory {
    pub name: Option<String>,
    pub description: Option<String>,
}
