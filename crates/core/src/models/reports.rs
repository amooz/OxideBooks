use serde::{Deserialize, Serialize};

use crate::{models::AccountType, money::MinorUnits};

/// Running debit/credit totals for one account over a set of posted journal entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    /// Sum of all debits posted to this account (minor units).
    pub debit_total: MinorUnits,
    /// Sum of all credits posted to this account (minor units).
    pub credit_total: MinorUnits,
}

impl AccountBalance {
    /// Net balance in the account's normal direction.
    ///
    /// Positive means the account has a balance in its normal direction
    /// (debit for assets/expenses, credit for liabilities/equity/revenue).
    /// Negative means a contra-balance.
    pub fn balance(&self) -> MinorUnits {
        if self.account_type.is_debit_normal() {
            self.debit_total - self.credit_total
        } else {
            self.credit_total - self.debit_total
        }
    }
}

/// Aggregated trial balance for an organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalance {
    pub accounts: Vec<AccountBalance>,
    /// Total of all debit postings across all accounts.
    pub total_debits: MinorUnits,
    /// Total of all credit postings across all accounts.
    pub total_credits: MinorUnits,
}

impl TrialBalance {
    /// True when the ledger satisfies the double-entry invariant (Σ debits == Σ credits).
    pub fn is_balanced(&self) -> bool {
        self.total_debits == self.total_credits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_balance(account_type: AccountType, debit: i64, credit: i64) -> AccountBalance {
        AccountBalance {
            account_id: "test-id".to_string(),
            account_code: "1000".to_string(),
            account_name: "Test Account".to_string(),
            account_type,
            debit_total: debit,
            credit_total: credit,
        }
    }

    #[test]
    fn asset_balance_is_debit_minus_credit() {
        let b = make_balance(AccountType::Asset, 10_000, 3_000);
        assert_eq!(b.balance(), 7_000);
    }

    #[test]
    fn expense_balance_is_debit_minus_credit() {
        let b = make_balance(AccountType::Expense, 5_000, 0);
        assert_eq!(b.balance(), 5_000);
    }

    #[test]
    fn liability_balance_is_credit_minus_debit() {
        let b = make_balance(AccountType::Liability, 3_000, 10_000);
        assert_eq!(b.balance(), 7_000);
    }

    #[test]
    fn equity_balance_is_credit_minus_debit() {
        let b = make_balance(AccountType::Equity, 0, 50_000);
        assert_eq!(b.balance(), 50_000);
    }

    #[test]
    fn revenue_balance_is_credit_minus_debit() {
        let b = make_balance(AccountType::Revenue, 1_000, 8_000);
        assert_eq!(b.balance(), 7_000);
    }

    #[test]
    fn contra_balance_is_negative() {
        // Asset with more credits than debits → negative balance
        let b = make_balance(AccountType::Asset, 1_000, 5_000);
        assert_eq!(b.balance(), -4_000);
    }

    #[test]
    fn zero_balance() {
        let b = make_balance(AccountType::Asset, 5_000, 5_000);
        assert_eq!(b.balance(), 0);
    }

    #[test]
    fn trial_balance_is_balanced() {
        let tb = TrialBalance {
            accounts: vec![],
            total_debits: 50_000,
            total_credits: 50_000,
        };
        assert!(tb.is_balanced());
    }

    #[test]
    fn trial_balance_not_balanced() {
        let tb = TrialBalance {
            accounts: vec![],
            total_debits: 50_000,
            total_credits: 49_999,
        };
        assert!(!tb.is_balanced());
    }

    #[test]
    fn trial_balance_zero() {
        let tb = TrialBalance {
            accounts: vec![],
            total_debits: 0,
            total_credits: 0,
        };
        assert!(tb.is_balanced());
    }
}
