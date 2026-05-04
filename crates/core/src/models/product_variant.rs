use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;

use crate::money::MinorUnits;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub organization_id: String,
    pub sku: Option<String>,
    pub name: String,
    /// Free-form key/value attributes, e.g. `{"size": "M", "color": "Red"}`.
    pub attributes: JsonValue,
    /// When `None`, the parent product's `unit_price` applies.
    pub price_override: Option<MinorUnits>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductVariant {
    pub sku: Option<String>,
    pub name: String,
    #[serde(default = "default_attributes")]
    pub attributes: JsonValue,
    pub price_override: Option<MinorUnits>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateProductVariant {
    pub sku: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<JsonValue>,
    pub price_override: Option<MinorUnits>,
    pub is_active: Option<bool>,
}

fn default_attributes() -> JsonValue {
    serde_json::json!({})
}
