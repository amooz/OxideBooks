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
    pub description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateAccount {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}
