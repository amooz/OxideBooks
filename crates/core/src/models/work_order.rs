use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct WorkOrder {
    pub id: String,
    pub organization_id: String,
    pub contact_id: Option<String>,
    pub assigned_to: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub scheduled_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub completed_date: Option<Date>,
    pub invoice_id: Option<String>,
    pub doc_number: Option<String>,
    pub notes: Option<String>,
    pub lines: Vec<WorkOrderLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct WorkOrderLine {
    pub id: String,
    pub work_order_id: String,
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i32,
    pub unit_price: i64,
    pub completed: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkOrder {
    pub title: String,
    pub contact_id: Option<String>,
    pub assigned_to: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub scheduled_date: Option<Date>,
    pub notes: Option<String>,
    #[serde(default)]
    pub lines: Vec<CreateWorkOrderLine>,
}

fn default_priority() -> String {
    "normal".into()
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkOrderLine {
    pub product_id: Option<String>,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub unit_price: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkOrder {
    pub title: Option<String>,
    pub contact_id: Option<String>,
    pub assigned_to: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub scheduled_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub completed_date: Option<Date>,
    pub notes: Option<String>,
}
