use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressClaim {
    pub id: String,
    pub organization_id: String,
    pub project_id: String,
    pub claim_number: i32,
    /// Percentage × 100 (e.g. 25% → 2500).
    pub claim_pct: i64,
    pub claim_amount: i64,
    pub retainage_pct: i64,
    pub retainage_amount: i64,
    pub net_amount: i64,
    pub status: String,
    pub notes: Option<String>,
    pub invoice_id: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub invoiced_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProgressClaim {
    /// Percentage of contract to claim in this draw (× 100; e.g. 25% → 2500).
    pub claim_pct: i64,
    /// Total claim amount in minor units.
    pub claim_amount: i64,
    /// Retainage holdback percentage (× 100).
    #[serde(default)]
    pub retainage_pct: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRetainage {
    pub notes: Option<String>,
}

// ── Project Billing Report ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBillingRow {
    pub project_id: String,
    pub project_name: String,
    pub billing_method: String,
    pub contract_amount: i64,
    pub billed_amount: i64,
    pub retainage_held: i64,
    pub unbilled_amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBillingReport {
    pub rows: Vec<ProjectBillingRow>,
    pub total_contract: i64,
    pub total_billed: i64,
    pub total_retainage: i64,
    pub total_unbilled: i64,
}
