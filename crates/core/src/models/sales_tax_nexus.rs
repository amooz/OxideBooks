use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct SalesTaxNexus {
    pub id: String,
    pub organization_id: String,
    pub jurisdiction_code: String,
    pub jurisdiction_name: String,
    pub nexus_type: String,
    pub registration_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub effective_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateSalesTaxNexus {
    pub jurisdiction_code: String,
    pub jurisdiction_name: String,
    pub nexus_type: Option<String>,
    pub registration_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub effective_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSalesTaxNexus {
    pub registration_number: Option<String>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub status: Option<String>,
    pub notes: Option<String>,
}
