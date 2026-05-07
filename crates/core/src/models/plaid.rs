use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaidItem {
    pub id: String,
    pub organization_id: String,
    pub bank_account_id: String,
    pub item_id: String,
    pub institution_id: Option<String>,
    pub institution_name: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_synced_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Request body to exchange a Plaid public token after Link completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangePlaidToken {
    pub public_token: String,
    pub bank_account_id: String,
    pub institution_id: Option<String>,
    pub institution_name: Option<String>,
}

/// Request body to manually trigger a sync for one or all items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaidSyncRequest {
    /// If provided, sync only this item; otherwise sync all active items.
    pub item_id: Option<String>,
}

/// Summary returned after a sync run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaidSyncResult {
    pub items_synced: usize,
    pub transactions_added: usize,
    pub transactions_skipped: usize,
}
