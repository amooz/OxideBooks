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

// ── Balance Sheet Comparison ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetComparisonSection {
    pub lines: Vec<ReportLine>,
    pub current_total: MinorUnits,
    pub prior_total: MinorUnits,
    pub change: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetComparisonReport {
    pub as_of: Date,
    pub prior_as_of: Date,
    pub assets: BalanceSheetComparisonSection,
    pub liabilities: BalanceSheetComparisonSection,
    pub equity: BalanceSheetComparisonSection,
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

// ── Cash Flow Forecast ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowForecastBucket {
    pub period_start: Date,
    pub period_end: Date,
    pub inflows: i64,
    pub outflows: i64,
    pub net: i64,
    pub running_balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowForecast {
    pub opening_balance: i64,
    pub buckets: Vec<CashFlowForecastBucket>,
    pub closing_balance: i64,
}

// ── Job Costing ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCostingCostCodeRow {
    pub cost_code_id: String,
    pub cost_code: String,
    pub cost_code_name: String,
    pub cost_type: String,
    pub actual_cost: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCostingRow {
    pub project_id: String,
    pub project_name: String,
    pub budget: MinorUnits,
    pub time_cost: MinorUnits,
    pub expense_cost: MinorUnits,
    pub bill_cost: MinorUnits,
    pub total_actual: MinorUnits,
    pub variance: MinorUnits,
    pub cost_codes: Vec<JobCostingCostCodeRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCostingReport {
    pub from: Date,
    pub to: Date,
    pub rows: Vec<JobCostingRow>,
    pub total_budget: MinorUnits,
    pub total_actual: MinorUnits,
    pub total_variance: MinorUnits,
}

// ── Vendor Spend ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorSpendRow {
    pub contact_id: String,
    pub vendor_name: String,
    pub bill_count: i64,
    pub subtotal: MinorUnits,
    pub tax_amount: MinorUnits,
    pub total_paid: MinorUnits,
    pub outstanding: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorSpendReport {
    pub from: Date,
    pub to: Date,
    pub rows: Vec<VendorSpendRow>,
    pub total_bills: i64,
    pub total_subtotal: MinorUnits,
    pub total_paid: MinorUnits,
    pub total_outstanding: MinorUnits,
}

// ── Payroll Summary ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollSummaryRow {
    pub run_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub period_start: Date,
    #[serde(with = "crate::models::date_serde")]
    pub period_end: Date,
    pub status: String,
    pub employee_count: i64,
    pub total_gross: MinorUnits,
    pub total_tax: MinorUnits,
    pub total_deductions: MinorUnits,
    pub total_net: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollSummaryReport {
    #[serde(with = "crate::models::date_serde")]
    pub from: Date,
    #[serde(with = "crate::models::date_serde")]
    pub to: Date,
    pub rows: Vec<PayrollSummaryRow>,
    pub total_gross: MinorUnits,
    pub total_tax: MinorUnits,
    pub total_net: MinorUnits,
}

// ── GRNI (Goods Received Not Invoiced) accrual ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrniRow {
    pub po_id: String,
    pub po_number: String,
    pub contact_id: String,
    pub contact_name: String,
    pub grn_id: String,
    #[serde(with = "crate::models::date_serde")]
    pub receipt_date: Date,
    pub product_description: String,
    pub quantity_received: i64,
    pub unit_cost: MinorUnits,
    pub accrual_amount: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrniReport {
    #[serde(with = "crate::models::date_serde")]
    pub as_of: Date,
    pub rows: Vec<GrniRow>,
    pub total_accrual: MinorUnits,
}

// ── W-2 / 941 payroll tax exports ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct W2Row {
    pub user_id: String,
    pub employee_name: String,
    pub email: String,
    pub year: i32,
    /// Box 1: wages, tips, other compensation
    pub wages: MinorUnits,
    /// Box 2: federal income tax withheld
    pub federal_income_tax_withheld: MinorUnits,
    /// Sum of other_deductions across all pay runs
    pub other_deductions: MinorUnits,
    pub net_pay: MinorUnits,
}

/// Aggregated payroll tax totals for a given quarter (Form 941 inputs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form941Quarter {
    pub year: i32,
    pub quarter: i32,
    /// Number of employees who received wages this quarter.
    pub employee_count: i64,
    /// Total wages, tips and other compensation (Line 2).
    pub wages: MinorUnits,
    /// Federal income tax withheld (Line 3).
    pub federal_income_tax: MinorUnits,
    /// Employee social security tax (Line 5a × 6.2%).
    pub social_security_employee: MinorUnits,
    /// Employer social security tax (Line 5a × 6.2%).
    pub social_security_employer: MinorUnits,
    /// Employee Medicare tax (Line 5c × 1.45%).
    pub medicare_employee: MinorUnits,
    /// Employer Medicare tax (Line 5c × 1.45%).
    pub medicare_employer: MinorUnits,
    /// Total deposits made for the quarter.
    pub total_deposits: MinorUnits,
}

// ── P&L Comparison ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PLComparisonReport {
    pub current: ProfitLossReport,
    pub prior: ProfitLossReport,
    /// current.net_income - prior.net_income
    pub net_income_change: MinorUnits,
    /// net_income_change as basis points of prior net_income (0 if prior is zero)
    pub net_income_change_bps: i64,
}

// ── AR / AP Aging Detail ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArAgingDetailRow {
    pub contact_id: String,
    pub contact_name: String,
    pub invoice_id: String,
    pub doc_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub invoice_date: Date,
    #[serde(with = "crate::models::opt_date_serde")]
    pub due_date: Option<Date>,
    pub total: MinorUnits,
    pub amount_paid: MinorUnits,
    pub balance: MinorUnits,
    pub days_overdue: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArAgingDetailReport {
    #[serde(with = "crate::models::date_serde")]
    pub as_of: Date,
    pub rows: Vec<ArAgingDetailRow>,
    pub total_outstanding: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApAgingDetailRow {
    pub contact_id: String,
    pub contact_name: String,
    pub bill_id: String,
    pub doc_number: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub bill_date: Date,
    #[serde(with = "crate::models::opt_date_serde")]
    pub due_date: Option<Date>,
    pub total: MinorUnits,
    pub amount_paid: MinorUnits,
    pub balance: MinorUnits,
    pub days_overdue: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApAgingDetailReport {
    #[serde(with = "crate::models::date_serde")]
    pub as_of: Date,
    pub rows: Vec<ApAgingDetailRow>,
    pub total_outstanding: MinorUnits,
}

// ── Sales by Customer ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesByCustomerRow {
    pub contact_id: String,
    pub contact_name: String,
    pub invoice_count: i64,
    pub total_invoiced: MinorUnits,
    pub total_paid: MinorUnits,
    pub balance_outstanding: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesByCustomerReport {
    #[serde(with = "crate::models::date_serde")]
    pub from: Date,
    #[serde(with = "crate::models::date_serde")]
    pub to: Date,
    pub rows: Vec<SalesByCustomerRow>,
    pub total_invoiced: MinorUnits,
    pub total_paid: MinorUnits,
}

// ── Remittance Advice ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceLine {
    pub bill_id: String,
    pub bill_number: Option<String>,
    pub bill_date: Date,
    pub original_amount: MinorUnits,
    pub amount_paid: MinorUnits,
    pub balance_remaining: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceAdvice {
    pub batch_payment_id: String,
    pub payment_date: Date,
    pub method: String,
    pub reference: Option<String>,
    pub payee_name: Option<String>,
    pub total_amount: MinorUnits,
    pub lines: Vec<RemittanceLine>,
}

// ── Outstanding Quotes ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutstandingQuoteRow {
    pub invoice_id: String,
    pub doc_number: Option<String>,
    pub contact_id: String,
    pub contact_name: Option<String>,
    #[serde(with = "crate::models::date_serde")]
    pub quote_date: Date,
    #[serde(with = "crate::models::opt_date_serde")]
    pub expiry_date: Option<Date>,
    pub status: String,
    pub total: MinorUnits,
    pub days_open: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutstandingQuotesReport {
    pub as_of: Date,
    pub rows: Vec<OutstandingQuoteRow>,
    pub total_value: MinorUnits,
}

// ── PO Spending by Vendor ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoSpendingRow {
    pub contact_id: String,
    pub vendor_name: Option<String>,
    pub po_count: i64,
    pub total_ordered: MinorUnits,
    pub total_received: MinorUnits,
    pub total_billed: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoSpendingReport {
    pub from: Date,
    pub to: Date,
    pub rows: Vec<PoSpendingRow>,
    pub total_ordered: MinorUnits,
    pub total_billed: MinorUnits,
}

// ── Indirect Cash Flow Statement ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowIndirectLine {
    pub account_id: String,
    pub account_code: String,
    pub account_name: String,
    pub net_change: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowIndirectSection {
    pub lines: Vec<CashFlowIndirectLine>,
    pub total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowIndirectReport {
    pub from: Date,
    pub to: Date,
    pub operating: CashFlowIndirectSection,
    pub investing: CashFlowIndirectSection,
    pub financing: CashFlowIndirectSection,
    pub net_change: MinorUnits,
}

// ── VAT Return ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatReturnLine {
    pub tax_rate_id: Option<String>,
    pub tax_rate_name: String,
    pub rate_pct: i64,
    pub taxable_sales: MinorUnits,
    pub output_tax: MinorUnits,
    pub taxable_purchases: MinorUnits,
    pub input_tax: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatReturnReport {
    pub from: Date,
    pub to: Date,
    pub lines: Vec<VatReturnLine>,
    pub total_output_tax: MinorUnits,
    pub total_input_tax: MinorUnits,
    pub net_vat_payable: MinorUnits,
}

// ── Sales Tax by Nexus ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTaxByNexusRow {
    pub jurisdiction_code: String,
    pub jurisdiction_name: String,
    pub taxable_sales: MinorUnits,
    pub tax_collected: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTaxByNexusReport {
    pub from: Date,
    pub to: Date,
    pub rows: Vec<SalesTaxByNexusRow>,
    pub total_taxable_sales: MinorUnits,
    pub total_tax_collected: MinorUnits,
}

// ── Currency Exposure ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyExposureRow {
    pub currency: String,
    pub ar_outstanding: MinorUnits,
    pub ap_outstanding: MinorUnits,
    pub net_exposure: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyExposureReport {
    pub as_of: Date,
    pub rows: Vec<CurrencyExposureRow>,
}

// ── Auto-Reversal Result ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReversalResult {
    pub reversed_count: i64,
    pub reversal_ids: Vec<String>,
}

// ── Cash Journals ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashReceiptsJournalRow {
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    pub contact_name: String,
    pub reference: Option<String>,
    pub payment_method: Option<String>,
    pub account_name: String,
    pub amount: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashReceiptsJournal {
    #[serde(with = "crate::models::date_serde")]
    pub from_date: Date,
    #[serde(with = "crate::models::date_serde")]
    pub to_date: Date,
    pub rows: Vec<CashReceiptsJournalRow>,
    pub total: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashDisbursementsJournalRow {
    #[serde(with = "crate::models::date_serde")]
    pub date: Date,
    pub contact_name: String,
    pub reference: Option<String>,
    pub payment_method: Option<String>,
    pub account_name: String,
    pub amount: MinorUnits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashDisbursementsJournal {
    #[serde(with = "crate::models::date_serde")]
    pub from_date: Date,
    #[serde(with = "crate::models::date_serde")]
    pub to_date: Date,
    pub rows: Vec<CashDisbursementsJournalRow>,
    pub total: MinorUnits,
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
