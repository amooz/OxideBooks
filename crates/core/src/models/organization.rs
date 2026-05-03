use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    /// ISO 4217 currency code (e.g. "USD")
    pub currency: String,
    /// Month number (1–12) the fiscal year begins
    pub fiscal_year_start: u8,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganization {
    pub name: String,
    pub currency: String,
    pub fiscal_year_start: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganization {
    pub name: Option<String>,
    pub currency: Option<String>,
    pub fiscal_year_start: Option<u8>,
}
