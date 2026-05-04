use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct ContractorTaxInfo {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub tax_id_type: String,
    pub tax_id_last4: String,
    pub business_type: String,
    pub form_1099_type: String,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub w9_received_date: Option<Date>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateContractorTaxInfo {
    pub contact_id: String,
    pub tax_id_type: Option<String>,
    pub tax_id_last4: String,
    pub business_type: Option<String>,
    pub form_1099_type: Option<String>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub w9_received_date: Option<Date>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContractorTaxInfo {
    pub tax_id_type: Option<String>,
    pub tax_id_last4: Option<String>,
    pub business_type: Option<String>,
    pub form_1099_type: Option<String>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub w9_received_date: Option<Date>,
    pub notes: Option<String>,
}

/// Aggregated payment totals for a contractor in a tax year.
#[derive(Debug, Serialize)]
pub struct Contractor1099Summary {
    pub contact_id: String,
    pub contact_name: String,
    pub form_1099_type: String,
    pub total_paid: i64,
    pub meets_threshold: bool,
}
