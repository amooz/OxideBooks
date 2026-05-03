use serde::{Deserialize, Serialize};
use time::Date;

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

// ── Profit & Loss ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportLine {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub amount: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub accounts: Vec<ReportLine>,
    pub total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitLossReport {
    pub from: Date,
    pub to: Date,
    pub revenue: ReportSection,
    pub expenses: ReportSection,
    pub net_income: MinorUnits,
}

// ── Balance Sheet ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetReport {
    pub as_of: Date,
    pub assets: ReportSection,
    pub liabilities: ReportSection,
    pub equity: ReportSection,
    pub is_balanced: bool,
}

// ── AR/AP Aging ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingRow {
    pub contact_id: String,
    pub contact_name: String,
    pub current: MinorUnits,
    pub days_1_30: MinorUnits,
    pub days_31_60: MinorUnits,
    pub days_61_90: MinorUnits,
    pub days_over_90: MinorUnits,
    pub total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingReport {
    pub as_of: Date,
    pub aging_type: String,
    pub rows: Vec<AgingRow>,
    pub totals: AgingRow,
}

// ── Dashboard KPIs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardKpis {
    pub cash_balance: MinorUnits,
    pub accounts_receivable: MinorUnits,
    pub accounts_payable: MinorUnits,
    pub revenue_mtd: MinorUnits,
    pub expenses_mtd: MinorUnits,
    pub overdue_invoices: i64,
}

// ── Cash Flow ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowSection {
    pub items: Vec<ReportLine>,
    pub total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowReport {
    pub from: Date,
    pub to: Date,
    pub operating: CashFlowSection,
    pub investing: CashFlowSection,
    pub financing: CashFlowSection,
    pub net_change: MinorUnits,
    pub opening_cash: MinorUnits,
    pub closing_cash: MinorUnits,
}

// ── Tax Summary ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSummaryLine {
    pub tax_rate_id: String,
    pub tax_rate_name: String,
    pub rate_bps: i32,
    pub tax_collected: MinorUnits,
    pub tax_paid: MinorUnits,
    pub net: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSummaryReport {
    pub from: Date,
    pub to: Date,
    pub lines: Vec<TaxSummaryLine>,
    pub total_collected: MinorUnits,
    pub total_paid: MinorUnits,
    pub net: MinorUnits,
}

// ── 1099 Summary ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor1099Row {
    pub contact_id: String,
    pub contact_name: String,
    pub tax_id: Option<String>,
    pub total_paid: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary1099 {
    pub year: i32,
    pub vendors: Vec<Vendor1099Row>,
}

// ── Account Ledger ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerLine {
    pub journal_entry_id: String,
    pub date: Date,
    pub description: String,
    pub reference: Option<String>,
    pub debit: MinorUnits,
    pub credit: MinorUnits,
    pub running_balance: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountLedger {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub from: Date,
    pub to: Date,
    pub opening_balance: MinorUnits,
    pub lines: Vec<LedgerLine>,
    pub closing_balance: MinorUnits,
}

// ── Project Profitability ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfitabilityRow {
    pub project_id: String,
    pub project_name: String,
    pub invoiced_amount: MinorUnits,
    pub expense_amount: MinorUnits,
    pub time_cost: MinorUnits,
    pub gross_profit: MinorUnits,
    pub margin_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfitabilityReport {
    pub rows: Vec<ProjectProfitabilityRow>,
    pub total_invoiced: MinorUnits,
    pub total_expenses: MinorUnits,
    pub total_time_cost: MinorUnits,
    pub total_profit: MinorUnits,
}

// ── Sales by Product ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesByProductRow {
    pub product_id: String,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity: i64,
    pub gross_amount: MinorUnits,
    pub discount_amount: MinorUnits,
    pub net_amount: MinorUnits,
    pub tax_amount: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesByProductReport {
    pub from: Date,
    pub to: Date,
    pub rows: Vec<SalesByProductRow>,
    pub total_gross: MinorUnits,
    pub total_discount: MinorUnits,
    pub total_net: MinorUnits,
    pub total_tax: MinorUnits,
}

// ── Global Search ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub display: String,
    #[serde(rename = "type")]
    pub hit_type: String,
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
