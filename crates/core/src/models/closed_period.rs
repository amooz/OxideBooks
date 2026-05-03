use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedPeriod {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
    pub closed_by: Option<String>,
    pub notes: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub closed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClosedPeriod {
    #[serde(with = "date_serde")]
    pub period_start: Date,
    #[serde(with = "date_serde")]
    pub period_end: Date,
    pub notes: Option<String>,
}
