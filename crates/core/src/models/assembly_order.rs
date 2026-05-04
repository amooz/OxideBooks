use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct AssemblyOrder {
    pub id: String,
    pub organization_id: String,
    pub product_id: String,
    pub quantity: i32,
    pub status: String,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub build_date: Option<Date>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct AssemblyOrderLine {
    pub id: String,
    pub assembly_order_id: String,
    pub component_id: String,
    pub quantity_required: i32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssemblyOrder {
    pub product_id: String,
    pub quantity: i32,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub build_date: Option<Date>,
    pub notes: Option<String>,
    pub components: Vec<CreateAssemblyOrderLine>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAssemblyOrderLine {
    pub component_id: String,
    pub quantity_required: i32,
}
