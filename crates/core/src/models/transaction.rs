use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::{money::MinorUnits, CoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Voided,
}

impl std::fmt::Display for JournalEntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JournalEntryStatus::Draft => "draft",
            JournalEntryStatus::Posted => "posted",
            JournalEntryStatus::Voided => "voided",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub organization_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    pub reference: Option<String>,
    pub description: String,
    pub status: JournalEntryStatus,
    pub lines: Vec<JournalLine>,
    pub created_by: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl JournalEntry {
    /// Double-entry invariant: total debits must equal total credits.
    pub fn is_balanced(&self) -> bool {
        let debits: i64 = self.lines.iter().map(|l| l.debit).sum();
        let credits: i64 = self.lines.iter().map(|l| l.credit).sum();
        debits == credits
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.lines.len() < 2 {
            return Err(CoreError::InsufficientLines);
        }
        let debits: i64 = self.lines.iter().map(|l| l.debit).sum();
        let credits: i64 = self.lines.iter().map(|l| l.credit).sum();
        if debits != credits {
            return Err(CoreError::UnbalancedEntry { debits, credits });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalLine {
    pub id: String,
    pub journal_entry_id: String,
    pub account_id: String,
    pub description: Option<String>,
    /// Amount in minor units (e.g. cents). Exactly one of debit/credit is non-zero.
    pub debit: MinorUnits,
    pub credit: MinorUnits,
}

#[derive(Debug, Deserialize)]
pub struct CreateJournalEntry {
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    pub reference: Option<String>,
    pub description: String,
    pub lines: Vec<CreateJournalLine>,
}

impl CreateJournalEntry {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.lines.len() < 2 {
            return Err(CoreError::InsufficientLines);
        }
        let debits: i64 = self.lines.iter().map(|l| l.debit).sum();
        let credits: i64 = self.lines.iter().map(|l| l.credit).sum();
        if debits != credits {
            return Err(CoreError::UnbalancedEntry { debits, credits });
        }
        for line in &self.lines {
            if line.debit < 0 || line.credit < 0 {
                return Err(CoreError::NegativeAmount(line.debit.min(line.credit)));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateJournalLine {
    pub account_id: String,
    pub description: Option<String>,
    pub debit: MinorUnits,
    pub credit: MinorUnits,
}
