use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::models::date_serde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankFeedTransaction {
    pub id: String,
    pub organization_id: String,
    pub bank_account_id: String,
    #[serde(with = "date_serde")]
    pub txn_date: Date,
    pub description: String,
    pub amount: i64,
    pub txn_type: String,
    pub reference: Option<String>,
    pub source: String,
    pub status: String,
    pub matched_txn_id: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// A single row from a CSV import. The caller parses the CSV and passes rows here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBankFeedRow {
    #[serde(with = "date_serde")]
    pub txn_date: Date,
    pub description: String,
    /// Positive = credit (money in), negative = debit (money out).
    pub amount: i64,
    pub reference: Option<String>,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "csv".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBankFeed {
    pub rows: Vec<ImportBankFeedRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchBankFeedTransaction {
    /// The existing `bank_transactions.id` to link this feed entry to.
    pub bank_transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankFeedAutoMatchResult {
    pub matched: usize,
    pub unmatched: usize,
}
