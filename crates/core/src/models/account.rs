use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl AccountType {
    /// Returns true when debits increase this account's balance (debit-normal).
    pub fn is_debit_normal(self) -> bool {
        matches!(self, AccountType::Asset | AccountType::Expense)
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AccountType::Asset => "asset",
            AccountType::Liability => "liability",
            AccountType::Equity => "equity",
            AccountType::Revenue => "revenue",
            AccountType::Expense => "expense",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for AccountType {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asset" => Ok(AccountType::Asset),
            "liability" => Ok(AccountType::Liability),
            "equity" => Ok(AccountType::Equity),
            "revenue" => Ok(AccountType::Revenue),
            "expense" => Ok(AccountType::Expense),
            _ => Err(CoreError::UnknownAccountType(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub organization_id: String,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub parent_id: Option<String>,
    pub sub_type: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccount {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub parent_id: Option<String>,
    pub sub_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateAccount {
    pub code: Option<String>,
    pub name: Option<String>,
    pub sub_type: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // ── AccountType::is_debit_normal ──────────────────────────────────────────

    #[test]
    fn asset_is_debit_normal() {
        assert!(AccountType::Asset.is_debit_normal());
    }

    #[test]
    fn expense_is_debit_normal() {
        assert!(AccountType::Expense.is_debit_normal());
    }

    #[test]
    fn liability_is_credit_normal() {
        assert!(!AccountType::Liability.is_debit_normal());
    }

    #[test]
    fn equity_is_credit_normal() {
        assert!(!AccountType::Equity.is_debit_normal());
    }

    #[test]
    fn revenue_is_credit_normal() {
        assert!(!AccountType::Revenue.is_debit_normal());
    }

    // ── AccountType Display ───────────────────────────────────────────────────

    #[test]
    fn display_all_variants() {
        assert_eq!(AccountType::Asset.to_string(), "asset");
        assert_eq!(AccountType::Liability.to_string(), "liability");
        assert_eq!(AccountType::Equity.to_string(), "equity");
        assert_eq!(AccountType::Revenue.to_string(), "revenue");
        assert_eq!(AccountType::Expense.to_string(), "expense");
    }

    // ── AccountType FromStr ───────────────────────────────────────────────────

    #[test]
    fn from_str_valid() {
        assert_eq!(AccountType::from_str("asset").unwrap(), AccountType::Asset);
        assert_eq!(
            AccountType::from_str("liability").unwrap(),
            AccountType::Liability
        );
        assert_eq!(
            AccountType::from_str("equity").unwrap(),
            AccountType::Equity
        );
        assert_eq!(
            AccountType::from_str("revenue").unwrap(),
            AccountType::Revenue
        );
        assert_eq!(
            AccountType::from_str("expense").unwrap(),
            AccountType::Expense
        );
    }

    #[test]
    fn from_str_case_sensitive() {
        assert!(AccountType::from_str("Asset").is_err());
        assert!(AccountType::from_str("ASSET").is_err());
        assert!(AccountType::from_str("Liability").is_err());
    }

    #[test]
    fn from_str_unknown() {
        assert!(AccountType::from_str("").is_err());
        assert!(AccountType::from_str("bank").is_err());
        assert!(AccountType::from_str("income").is_err());
    }

    // ── Roundtrip (Display → FromStr) ─────────────────────────────────────────

    #[test]
    fn display_then_parse_roundtrip() {
        for t in [
            AccountType::Asset,
            AccountType::Liability,
            AccountType::Equity,
            AccountType::Revenue,
            AccountType::Expense,
        ] {
            let s = t.to_string();
            let parsed = AccountType::from_str(&s).unwrap();
            assert_eq!(parsed, t);
        }
    }

    // ── Serde roundtrip ───────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let t = AccountType::Revenue;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#""revenue""#);
        let parsed: AccountType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }
}
