use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct ConsolidationElimination {
    pub id: String,
    pub organization_id: String,
    pub intercompany_link_id: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub period_start: Date,
    #[serde(with = "crate::models::date_serde")]
    pub period_end: Date,
    pub debit_account_id: String,
    pub credit_account_id: String,
    pub amount: i64,
    pub description: String,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateConsolidationElimination {
    pub intercompany_link_id: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub period_start: Date,
    #[serde(with = "crate::models::date_serde")]
    pub period_end: Date,
    pub debit_account_id: String,
    pub credit_account_id: String,
    pub amount: i64,
    #[serde(default)]
    pub description: String,
    pub notes: Option<String>,
}
