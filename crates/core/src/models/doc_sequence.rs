use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSequence {
    pub id: String,
    pub organization_id: String,
    pub doc_type: String,
    pub prefix: String,
    pub next_number: i64,
    pub pad_length: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpsertDocSequence {
    pub doc_type: String,
    pub prefix: Option<String>,
    pub pad_length: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ResetDocSequence {
    pub next_number: i64,
}
