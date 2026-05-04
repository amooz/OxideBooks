use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercompanyLink {
    pub id: String,
    pub organization_id: String,
    pub counterparty_org_id: String,
    pub due_from_account_id: Option<String>,
    pub due_to_account_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateIntercompanyLink {
    pub counterparty_org_id: String,
    pub due_from_account_id: Option<String>,
    pub due_to_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntercompanyTransaction {
    pub id: String,
    pub org_a_id: String,
    pub journal_entry_a: String,
    pub org_b_id: String,
    pub journal_entry_b: String,
    pub amount: i64,
    pub currency: String,
    pub description: Option<String>,
    #[serde(with = "date_serde")]
    pub transaction_date: Date,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Creates symmetric journal entries in two organisations simultaneously.
/// The caller must have admin access to the requesting org (org_a); the
/// counterparty org_id is taken from the matching intercompany_link.
#[derive(Debug, Deserialize)]
pub struct CreateIntercompanyTransaction {
    /// Must match an existing intercompany_link.counterparty_org_id
    pub counterparty_org_id: String,
    #[serde(with = "date_serde")]
    pub transaction_date: Date,
    pub amount: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub description: Option<String>,
    /// Account to debit in the initiating org (org_a)
    pub debit_account_id_a: String,
    /// Account to credit in the initiating org (org_a)
    pub credit_account_id_a: String,
    /// Account to debit in the counterparty org (org_b)
    pub debit_account_id_b: String,
    /// Account to credit in the counterparty org (org_b)
    pub credit_account_id_b: String,
}

fn default_currency() -> String {
    "USD".into()
}
