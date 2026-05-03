use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LateFeeRule {
    pub id: String,
    pub organization_id: String,
    pub grace_days: i32,
    pub fee_type: String,
    pub amount: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertLateFeeRule {
    #[serde(default)]
    pub grace_days: i32,
    pub fee_type: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LateFee {
    pub id: String,
    pub invoice_id: String,
    pub organization_id: String,
    pub amount: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub applied_at: OffsetDateTime,
}
