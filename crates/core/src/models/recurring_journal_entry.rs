use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct RecurringJournalEntry {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    pub description: Option<String>,
    pub frequency: String,
    #[serde(with = "crate::models::date_serde")]
    pub next_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub is_active: bool,
    pub auto_post: bool,
    pub lines: Vec<RecurringJournalEntryLine>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct RecurringJournalEntryLine {
    pub id: String,
    pub recurring_journal_entry_id: String,
    pub account_id: String,
    pub description: Option<String>,
    pub debit: i64,
    pub credit: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecurringJournalEntry {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_frequency")]
    pub frequency: String,
    #[serde(with = "crate::models::date_serde")]
    pub next_date: Date,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    #[serde(default)]
    pub auto_post: bool,
    pub lines: Vec<CreateRecurringJournalEntryLine>,
}

fn default_frequency() -> String {
    "monthly".into()
}

#[derive(Debug, Deserialize)]
pub struct CreateRecurringJournalEntryLine {
    pub account_id: String,
    pub description: Option<String>,
    #[serde(default)]
    pub debit: i64,
    #[serde(default)]
    pub credit: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecurringJournalEntry {
    pub name: Option<String>,
    pub description: Option<String>,
    pub frequency: Option<String>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub next_date: Option<Date>,
    #[serde(default, with = "crate::models::opt_date_serde")]
    pub end_date: Option<Date>,
    pub is_active: Option<bool>,
    pub auto_post: Option<bool>,
}
