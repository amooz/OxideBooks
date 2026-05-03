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
    pub reversal_of: Option<String>,
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
        // Validate individual lines first so sums are meaningful.
        for line in &self.lines {
            if line.debit < 0 || line.credit < 0 {
                return Err(CoreError::NegativeAmount(line.debit.min(line.credit)));
            }
            if line.debit > 0 && line.credit > 0 {
                return Err(CoreError::BothDebitAndCredit);
            }
        }
        let debits: i64 = self.lines.iter().map(|l| l.debit).sum();
        let credits: i64 = self.lines.iter().map(|l| l.credit).sum();
        if debits != credits {
            return Err(CoreError::UnbalancedEntry { debits, credits });
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn date() -> Date {
        Date::from_calendar_date(2025, Month::January, 15).unwrap()
    }

    fn debit_line(account_id: &str, amount: i64) -> CreateJournalLine {
        CreateJournalLine {
            account_id: account_id.to_string(),
            description: None,
            debit: amount,
            credit: 0,
        }
    }

    fn credit_line(account_id: &str, amount: i64) -> CreateJournalLine {
        CreateJournalLine {
            account_id: account_id.to_string(),
            description: None,
            debit: 0,
            credit: amount,
        }
    }

    fn simple_entry(amount: i64) -> CreateJournalEntry {
        CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Test".to_string(),
            lines: vec![debit_line("cash", amount), credit_line("revenue", amount)],
        }
    }

    // ── CreateJournalEntry::validate ──────────────────────────────────────────

    #[test]
    fn valid_two_line_entry() {
        assert!(simple_entry(10_000).validate().is_ok());
    }

    #[test]
    fn valid_multi_line_entry() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: Some("REF-001".to_string()),
            description: "Split".to_string(),
            lines: vec![
                debit_line("cash", 6_000),
                debit_line("bank", 4_000),
                credit_line("revenue", 10_000),
            ],
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn rejects_single_line() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![debit_line("cash", 100)],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::InsufficientLines)
        ));
    }

    #[test]
    fn rejects_empty_lines() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::InsufficientLines)
        ));
    }

    #[test]
    fn rejects_unbalanced() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Unbalanced".to_string(),
            lines: vec![debit_line("cash", 10_000), credit_line("revenue", 9_000)],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::UnbalancedEntry {
                debits: 10_000,
                credits: 9_000
            })
        ));
    }

    #[test]
    fn rejects_negative_debit() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![
                CreateJournalLine {
                    account_id: "a".to_string(),
                    description: None,
                    debit: -100,
                    credit: 0,
                },
                credit_line("b", 100),
            ],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::NegativeAmount(_))
        ));
    }

    #[test]
    fn rejects_negative_credit() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![
                debit_line("a", 100),
                CreateJournalLine {
                    account_id: "b".to_string(),
                    description: None,
                    debit: 0,
                    credit: -100,
                },
            ],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::NegativeAmount(_))
        ));
    }

    #[test]
    fn rejects_both_debit_and_credit_on_same_line() {
        let entry = CreateJournalEntry {
            date: date(),
            reference: None,
            description: "Bad".to_string(),
            lines: vec![
                CreateJournalLine {
                    account_id: "a".to_string(),
                    description: None,
                    debit: 100,
                    credit: 100,
                },
                credit_line("b", 0),
            ],
        };
        assert!(matches!(
            entry.validate(),
            Err(CoreError::BothDebitAndCredit)
        ));
    }

    // ── JournalEntry::is_balanced ─────────────────────────────────────────────

    #[test]
    fn status_display() {
        assert_eq!(JournalEntryStatus::Draft.to_string(), "draft");
        assert_eq!(JournalEntryStatus::Posted.to_string(), "posted");
        assert_eq!(JournalEntryStatus::Voided.to_string(), "voided");
    }
}
