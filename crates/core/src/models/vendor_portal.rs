use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorPortalToken {
    pub id: String,
    pub token: String,
    pub contact_id: String,
    pub organization_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVendorPortalToken {
    pub contact_id: String,
    #[serde(default = "default_expiry_hours")]
    pub expires_hours: i64,
}

fn default_expiry_hours() -> i64 {
    168 // 7 days default for vendors
}
