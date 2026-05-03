use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankRule {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub match_field: String,
    pub match_type: String,
    pub match_value: String,
    pub account_id: String,
    pub auto_description: Option<String>,
    pub priority: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateBankRule {
    pub name: String,
    pub match_field: String,
    pub match_type: String,
    pub match_value: String,
    pub account_id: String,
    pub auto_description: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    100
}
