use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retainer {
    pub id: String,
    pub organization_id: String,
    pub contact_id: String,
    pub currency: String,
    pub balance_cents: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRetainer {
    pub contact_id: String,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainerTransaction {
    pub id: String,
    pub retainer_id: String,
    pub invoice_id: Option<String>,
    pub amount: i64,
    pub txn_type: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRetainer {
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRetainer {
    pub invoice_id: String,
    pub amount: i64,
}
