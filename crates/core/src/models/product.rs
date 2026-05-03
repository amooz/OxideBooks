use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleComponent {
    pub id: String,
    pub product_id: String,
    pub component_id: String,
    pub component_name: String,
    pub quantity: i64,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub sku: Option<String>,
    pub unit_price: i64,
    pub currency: String,
    pub account_id: Option<String>,
    pub tax_rate_id: Option<String>,
    pub category_id: Option<String>,
    pub is_active: bool,
    pub is_bundle: bool,
    #[serde(default)]
    pub bundle_components: Vec<BundleComponent>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    pub description: Option<String>,
    pub sku: Option<String>,
    #[serde(default)]
    pub unit_price: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub account_id: Option<String>,
    pub tax_rate_id: Option<String>,
    pub category_id: Option<String>,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sku: Option<String>,
    pub unit_price: Option<i64>,
    pub account_id: Option<String>,
    pub tax_rate_id: Option<String>,
    pub category_id: Option<String>,
    pub is_active: Option<bool>,
    pub is_bundle: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleComponentInput {
    pub component_id: String,
    pub quantity: i64,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBundleComponents {
    pub components: Vec<BundleComponentInput>,
}
