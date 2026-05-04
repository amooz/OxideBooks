use oxidebooks_core::models::{
    AccountBalance, AccountLedger, AccountType, AgingReport, AgingRow, BalanceSheetReport,
    CashFlowForecast, CashFlowForecastBucket, CashFlowReport, CashFlowSection,
    ConsolidatedProfitLoss, ContactStatement, DashboardKpis, Form941Quarter, GrniReport, GrniRow,
    JobCostingCostCodeRow, JobCostingReport, JobCostingRow, LedgerLine, OrgProfitLoss,
    PLComparisonReport, PayrollSummaryReport, PayrollSummaryRow, ProfitLossReport,
    ProjectProfitabilityReport, ProjectProfitabilityRow, ReportLine, ReportSection,
    SalesByProductReport, SalesByProductRow, StatementLine, TaxSummaryLine, TaxSummaryReport,
    TrialBalance, VendorSpendReport, VendorSpendRow, W2Row,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::Date;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct AccountBalanceRow {
    account_id: Uuid,
    account_code: String,
    account_name: String,
    account_type: String,
    debit_total: i64,
    credit_total: i64,
}

pub struct ReportRepo;

impl ReportRepo {
    /// Compute the trial balance for an organization.
    ///
    /// Only `posted` journal entries are included. Accounts with no activity
    /// are included with zero totals so the chart of accounts is fully visible.
    pub async fn trial_balance(pool: &PgPool, org_id: &str) -> Result<TrialBalance, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<AccountBalanceRow> = sqlx::query_as(
            r#"
            SELECT
                a.id          AS account_id,
                a.code        AS account_code,
                a.name        AS account_name,
                a.account_type,
                COALESCE(SUM(jl.debit),  0)::BIGINT AS debit_total,
                COALESCE(SUM(jl.credit), 0)::BIGINT AS credit_total
            FROM accounts a
            LEFT JOIN journal_lines jl ON jl.account_id = a.id
            LEFT JOIN journal_entries je
                ON  je.id              = jl.journal_entry_id
                AND je.organization_id = $1
                AND je.status          = 'posted'
            WHERE a.organization_id = $1
              AND a.is_active = TRUE
            GROUP BY a.id, a.code, a.name, a.account_type
            ORDER BY a.code
            "#,
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let accounts: Result<Vec<AccountBalance>, DbError> = rows
            .into_iter()
            .map(|r| {
                Ok(AccountBalance {
                    account_id: r.account_id.to_string(),
                    account_code: r.account_code,
                    account_name: r.account_name,
                    account_type: AccountType::from_str(&r.account_type)?,
                    debit_total: r.debit_total,
                    credit_total: r.credit_total,
                })
            })
            .collect();

        let accounts = accounts?;
        let total_debits = accounts.iter().map(|a| a.debit_total).sum();
        let total_credits = accounts.iter().map(|a| a.credit_total).sum();

        Ok(TrialBalance {
            accounts,
            total_debits,
            total_credits,
        })
    }

    /// Profit & Loss (Income Statement) for a date range.
    pub async fn profit_loss(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<ProfitLossReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: Uuid,
            account_code: String,
            account_name: String,
            account_type: String,
            total_debit: i64,
            total_credit: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                a.id          AS account_id,
                a.code        AS account_code,
                a.name        AS account_name,
                a.account_type,
                COALESCE(SUM(jl.debit),  0)::BIGINT AS total_debit,
                COALESCE(SUM(jl.credit), 0)::BIGINT AS total_credit
            FROM accounts a
            LEFT JOIN journal_lines jl ON jl.account_id = a.id
            LEFT JOIN journal_entries je
                ON  je.id              = jl.journal_entry_id
                AND je.organization_id = $1
                AND je.status          = 'posted'
                AND je.date BETWEEN $2 AND $3
            WHERE a.organization_id = $1
              AND a.account_type IN ('revenue', 'expense')
            GROUP BY a.id, a.code, a.name, a.account_type
            ORDER BY a.code
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut revenue_lines = vec![];
        let mut expense_lines = vec![];

        for r in rows {
            let amount = match r.account_type.as_str() {
                "revenue" => r.total_credit - r.total_debit,
                _ => r.total_debit - r.total_credit,
            };
            let line = ReportLine {
                account_id: r.account_id.to_string(),
                account_code: r.account_code,
                account_name: r.account_name,
                amount,
            };
            if r.account_type == "revenue" {
                revenue_lines.push(line);
            } else {
                expense_lines.push(line);
            }
        }

        let revenue_total: i64 = revenue_lines.iter().map(|l| l.amount).sum();
        let expense_total: i64 = expense_lines.iter().map(|l| l.amount).sum();

        Ok(ProfitLossReport {
            from,
            to,
            revenue: ReportSection {
                accounts: revenue_lines,
                total: revenue_total,
            },
            expenses: ReportSection {
                accounts: expense_lines,
                total: expense_total,
            },
            net_income: revenue_total - expense_total,
        })
    }

    /// Balance Sheet as of a specific date (cumulative from inception).
    pub async fn balance_sheet(
        pool: &PgPool,
        org_id: &str,
        as_of: Date,
    ) -> Result<BalanceSheetReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: Uuid,
            account_code: String,
            account_name: String,
            account_type: String,
            total_debit: i64,
            total_credit: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                a.id          AS account_id,
                a.code        AS account_code,
                a.name        AS account_name,
                a.account_type,
                COALESCE(SUM(jl.debit),  0)::BIGINT AS total_debit,
                COALESCE(SUM(jl.credit), 0)::BIGINT AS total_credit
            FROM accounts a
            LEFT JOIN journal_lines jl ON jl.account_id = a.id
            LEFT JOIN journal_entries je
                ON  je.id              = jl.journal_entry_id
                AND je.organization_id = $1
                AND je.status          = 'posted'
                AND je.date            <= $2
            WHERE a.organization_id = $1
              AND a.account_type IN ('asset', 'liability', 'equity')
            GROUP BY a.id, a.code, a.name, a.account_type
            ORDER BY a.code
            "#,
        )
        .bind(org_uuid)
        .bind(as_of)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut asset_lines = vec![];
        let mut liability_lines = vec![];
        let mut equity_lines = vec![];

        for r in rows {
            let amount = match r.account_type.as_str() {
                "asset" => r.total_debit - r.total_credit,
                _ => r.total_credit - r.total_debit,
            };
            let line = ReportLine {
                account_id: r.account_id.to_string(),
                account_code: r.account_code,
                account_name: r.account_name,
                amount,
            };
            match r.account_type.as_str() {
                "asset" => asset_lines.push(line),
                "liability" => liability_lines.push(line),
                _ => equity_lines.push(line),
            }
        }

        let asset_total: i64 = asset_lines.iter().map(|l| l.amount).sum();
        let liability_total: i64 = liability_lines.iter().map(|l| l.amount).sum();
        let equity_total: i64 = equity_lines.iter().map(|l| l.amount).sum();

        Ok(BalanceSheetReport {
            as_of,
            assets: ReportSection {
                accounts: asset_lines,
                total: asset_total,
            },
            liabilities: ReportSection {
                accounts: liability_lines,
                total: liability_total,
            },
            equity: ReportSection {
                accounts: equity_lines,
                total: equity_total,
            },
            is_balanced: asset_total == liability_total + equity_total,
        })
    }

    /// AR/AP aging report — outstanding invoice balances bucketed by days overdue.
    pub async fn aging(
        pool: &PgPool,
        org_id: &str,
        aging_type: &str,
        as_of: Date,
    ) -> Result<AgingReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let invoice_type = if aging_type == "payable" {
            "bill"
        } else {
            "invoice"
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Uuid,
            contact_name: String,
            due_date: Date,
            balance: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                c.id   AS contact_id,
                c.name AS contact_name,
                i.due_date,
                GREATEST(0,
                    (SELECT COALESCE(SUM(il.quantity * il.unit_price
                           * (1 + il.tax_rate::float8 / 10000)), 0)::BIGINT
                     FROM invoice_lines il WHERE il.invoice_id = i.id)
                    - COALESCE((SELECT SUM(p.amount) FROM payments p
                                WHERE p.invoice_id = i.id
                                  AND p.payment_date <= $3), 0)
                ) AS balance
            FROM invoices i
            JOIN contacts c ON c.id = i.contact_id
            WHERE i.organization_id = $1
              AND i.invoice_type = $2
              AND i.status IN ('sent','partial')
              AND i.date <= $3
            "#,
        )
        .bind(org_uuid)
        .bind(invoice_type)
        .bind(as_of)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Bucket by days overdue
        use std::collections::HashMap;
        let mut map: HashMap<String, AgingRow> = HashMap::new();

        for row in &rows {
            let days_overdue = (as_of - row.due_date).whole_days();
            let entry = map
                .entry(row.contact_id.to_string())
                .or_insert_with(|| AgingRow {
                    contact_id: row.contact_id.to_string(),
                    contact_name: row.contact_name.clone(),
                    current: 0,
                    days_1_30: 0,
                    days_31_60: 0,
                    days_61_90: 0,
                    days_over_90: 0,
                    total: 0,
                });

            match days_overdue {
                d if d <= 0 => entry.current += row.balance,
                1..=30 => entry.days_1_30 += row.balance,
                31..=60 => entry.days_31_60 += row.balance,
                61..=90 => entry.days_61_90 += row.balance,
                _ => entry.days_over_90 += row.balance,
            }
            entry.total += row.balance;
        }

        let mut aging_rows: Vec<AgingRow> = map.into_values().collect();
        aging_rows.retain(|r| r.total > 0);
        aging_rows.sort_by(|a, b| a.contact_name.cmp(&b.contact_name));

        let totals = AgingRow {
            contact_id: String::new(),
            contact_name: "Total".to_string(),
            current: aging_rows.iter().map(|r| r.current).sum(),
            days_1_30: aging_rows.iter().map(|r| r.days_1_30).sum(),
            days_31_60: aging_rows.iter().map(|r| r.days_31_60).sum(),
            days_61_90: aging_rows.iter().map(|r| r.days_61_90).sum(),
            days_over_90: aging_rows.iter().map(|r| r.days_over_90).sum(),
            total: aging_rows.iter().map(|r| r.total).sum(),
        };

        Ok(AgingReport {
            as_of,
            aging_type: aging_type.to_string(),
            rows: aging_rows,
            totals,
        })
    }

    /// Dashboard KPI snapshot for an organization.
    pub async fn dashboard(pool: &PgPool, org_id: &str) -> Result<DashboardKpis, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let cash: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(jl.debit - jl.credit), 0)::BIGINT
            FROM accounts a
            JOIN journal_lines jl ON jl.account_id = a.id
            JOIN journal_entries je ON je.id = jl.journal_entry_id
            WHERE a.organization_id = $1
              AND a.account_type = 'asset'
              AND a.sub_type IN ('bank', 'cash')
              AND je.status = 'posted'
            "#,
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let ar: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(
                (SELECT COALESCE(SUM(il.quantity * il.unit_price
                       * (1 + il.tax_rate::float8 / 10000)), 0)::BIGINT
                 FROM invoice_lines il WHERE il.invoice_id = i.id)
                - COALESCE((SELECT SUM(p.amount) FROM payments p WHERE p.invoice_id = i.id), 0)
            ), 0)::BIGINT
            FROM invoices i
            WHERE i.organization_id = $1
              AND i.invoice_type = 'invoice'
              AND i.status IN ('sent', 'partial')
            "#,
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let ap: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(
                (SELECT COALESCE(SUM(il.quantity * il.unit_price
                       * (1 + il.tax_rate::float8 / 10000)), 0)::BIGINT
                 FROM invoice_lines il WHERE il.invoice_id = i.id)
                - COALESCE((SELECT SUM(p.amount) FROM payments p WHERE p.invoice_id = i.id), 0)
            ), 0)::BIGINT
            FROM invoices i
            WHERE i.organization_id = $1
              AND i.invoice_type = 'bill'
              AND i.status IN ('sent', 'partial')
            "#,
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let revenue_mtd: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(jl.credit - jl.debit), 0)::BIGINT
            FROM accounts a
            JOIN journal_lines jl ON jl.account_id = a.id
            JOIN journal_entries je ON je.id = jl.journal_entry_id
            WHERE a.organization_id = $1
              AND a.account_type = 'revenue'
              AND je.status = 'posted'
              AND date_trunc('month', je.date::timestamptz) = date_trunc('month', NOW())
            "#,
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let expenses_mtd: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(jl.debit - jl.credit), 0)::BIGINT
            FROM accounts a
            JOIN journal_lines jl ON jl.account_id = a.id
            JOIN journal_entries je ON je.id = jl.journal_entry_id
            WHERE a.organization_id = $1
              AND a.account_type = 'expense'
              AND je.status = 'posted'
              AND date_trunc('month', je.date::timestamptz) = date_trunc('month', NOW())
            "#,
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let overdue: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoices i \
             WHERE i.organization_id = $1 \
               AND i.status IN ('sent','partial') \
               AND i.due_date < CURRENT_DATE",
        )
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(DashboardKpis {
            cash_balance: cash,
            accounts_receivable: ar,
            accounts_payable: ap,
            revenue_mtd,
            expenses_mtd,
            overdue_invoices: overdue,
        })
    }

    /// Tax summary report — tax collected on invoices vs paid on bills.
    pub async fn tax_summary(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<TaxSummaryReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            tax_rate_id: Uuid,
            tax_rate_name: String,
            rate_bps: i32,
            tax_collected: i64,
            tax_paid: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                tr.id           AS tax_rate_id,
                tr.name         AS tax_rate_name,
                tr.rate_bps,
                COALESCE(SUM(CASE WHEN i.invoice_type = 'invoice'
                    THEN (il.quantity * il.unit_price * il.tax_rate / 10000)::BIGINT ELSE 0 END), 0) AS tax_collected,
                COALESCE(SUM(CASE WHEN i.invoice_type = 'bill'
                    THEN (il.quantity * il.unit_price * il.tax_rate / 10000)::BIGINT ELSE 0 END), 0) AS tax_paid
            FROM tax_rates tr
            JOIN invoice_lines il ON il.tax_rate = tr.rate_bps
            JOIN invoices i ON i.id = il.invoice_id
            WHERE tr.organization_id = $1
              AND i.organization_id  = $1
              AND i.status != 'voided'
              AND i.date BETWEEN $2 AND $3
            GROUP BY tr.id, tr.name, tr.rate_bps
            ORDER BY tr.name
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let lines: Vec<TaxSummaryLine> = rows
            .iter()
            .map(|r| TaxSummaryLine {
                tax_rate_id: r.tax_rate_id.to_string(),
                tax_rate_name: r.tax_rate_name.clone(),
                rate_bps: r.rate_bps,
                tax_collected: r.tax_collected,
                tax_paid: r.tax_paid,
                net: r.tax_collected - r.tax_paid,
            })
            .collect();

        let total_collected = lines.iter().map(|l| l.tax_collected).sum();
        let total_paid = lines.iter().map(|l| l.tax_paid).sum();

        Ok(TaxSummaryReport {
            from,
            to,
            lines,
            total_collected,
            total_paid,
            net: total_collected - total_paid,
        })
    }

    /// Indirect-method cash flow statement.
    ///
    /// Starts from net income, then adjusts for non-cash items and working-capital
    /// changes. Investing = asset account net movements. Financing = liability/equity
    /// net movements excluding revenue/expense (already in operating).
    pub async fn cash_flow(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<CashFlowReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: Uuid,
            account_code: String,
            account_name: String,
            account_type: String,
            net: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                a.id          AS account_id,
                a.code        AS account_code,
                a.name        AS account_name,
                a.account_type,
                COALESCE(SUM(jl.debit - jl.credit), 0)::BIGINT AS net
            FROM accounts a
            JOIN journal_lines jl ON jl.account_id = a.id
            JOIN journal_entries je ON je.id = jl.journal_entry_id
            WHERE a.organization_id = $1
              AND je.organization_id = $1
              AND je.status = 'posted'
              AND je.date BETWEEN $2 AND $3
            GROUP BY a.id, a.code, a.name, a.account_type
            ORDER BY a.code
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Opening cash = sum of cash/bank asset accounts before `from`
        let opening_cash: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(jl.debit - jl.credit), 0)::BIGINT
            FROM accounts a
            JOIN journal_lines jl ON jl.account_id = a.id
            JOIN journal_entries je ON je.id = jl.journal_entry_id
            WHERE a.organization_id = $1
              AND a.account_type = 'asset'
              AND a.sub_type IN ('bank', 'cash')
              AND je.status = 'posted'
              AND je.date < $2
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut operating_items = vec![];
        let mut investing_items = vec![];
        let mut financing_items = vec![];

        for r in &rows {
            let amount = match r.account_type.as_str() {
                "revenue" => -(r.net), // credit normal → flip for cash impact
                "expense" => r.net,
                "asset" => -(r.net),             // debit increase = cash out
                "liability" | "equity" => r.net, // credit increase = cash in
                _ => 0,
            };
            if amount == 0 {
                continue;
            }
            let line = ReportLine {
                account_id: r.account_id.to_string(),
                account_code: r.account_code.clone(),
                account_name: r.account_name.clone(),
                amount,
            };
            match r.account_type.as_str() {
                "revenue" | "expense" => operating_items.push(line),
                "asset" => investing_items.push(line),
                _ => financing_items.push(line),
            }
        }

        let op_total: i64 = operating_items.iter().map(|l| l.amount).sum();
        let inv_total: i64 = investing_items.iter().map(|l| l.amount).sum();
        let fin_total: i64 = financing_items.iter().map(|l| l.amount).sum();
        let net_change = op_total + inv_total + fin_total;

        Ok(CashFlowReport {
            from,
            to,
            operating: CashFlowSection {
                items: operating_items,
                total: op_total,
            },
            investing: CashFlowSection {
                items: investing_items,
                total: inv_total,
            },
            financing: CashFlowSection {
                items: financing_items,
                total: fin_total,
            },
            net_change,
            opening_cash,
            closing_cash: opening_cash + net_change,
        })
    }

    /// Cash-basis P&L: revenue = payments received, expenses = reimbursed expenses.
    pub async fn profit_loss_cash(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<ProfitLossReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let total_revenue: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(p.amount), 0)::BIGINT \
             FROM payments p \
             JOIN invoices inv ON inv.id = p.invoice_id \
             WHERE inv.organization_id = $1 \
               AND p.payment_date BETWEEN $2 AND $3 \
               AND inv.invoice_type = 'invoice'",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        #[derive(sqlx::FromRow)]
        struct CategoryRow {
            category: String,
            total: i64,
        }
        let expense_rows: Vec<CategoryRow> = sqlx::query_as(
            "SELECT category, COALESCE(SUM(amount), 0)::BIGINT AS total \
             FROM expenses \
             WHERE organization_id = $1 \
               AND status = 'reimbursed' \
               AND expense_date BETWEEN $2 AND $3 \
             GROUP BY category ORDER BY total DESC",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let expense_total: i64 = expense_rows.iter().map(|r| r.total).sum();
        let expense_lines: Vec<ReportLine> = expense_rows
            .into_iter()
            .map(|r| ReportLine {
                account_id: String::new(),
                account_code: String::new(),
                account_name: r.category,
                amount: r.total,
            })
            .collect();

        Ok(ProfitLossReport {
            from,
            to,
            revenue: ReportSection {
                accounts: vec![ReportLine {
                    account_id: String::new(),
                    account_code: String::new(),
                    account_name: "Cash Receipts".to_string(),
                    amount: total_revenue,
                }],
                total: total_revenue,
            },
            expenses: ReportSection {
                accounts: expense_lines,
                total: expense_total,
            },
            net_income: total_revenue - expense_total,
        })
    }

    pub async fn contact_statement(
        pool: &PgPool,
        org_id: &str,
        contact_id: &str,
        from: Date,
        to: Date,
    ) -> Result<ContactStatement, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(contact_id)?;

        // Opening balance: (invoices issued before `from`) - (payments received before `from`)
        let inv_before: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(\
               (il.quantity * il.unit_price / 100) + \
               (il.quantity * il.unit_price / 100 * il.tax_rate / 10000)\
             ), 0)::BIGINT \
             FROM invoices inv \
             JOIN invoice_lines il ON il.invoice_id = inv.id \
             WHERE inv.organization_id = $1 AND inv.contact_id = $2 \
               AND inv.date < $3 AND inv.invoice_type = 'invoice' \
               AND inv.status NOT IN ('draft', 'voided')",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(from)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let pay_before: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(p.amount), 0)::BIGINT \
             FROM payments p \
             JOIN invoices inv ON inv.id = p.invoice_id \
             WHERE inv.organization_id = $1 AND inv.contact_id = $2 \
               AND p.payment_date < $3 AND inv.invoice_type = 'invoice'",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(from)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let opening_balance = inv_before - pay_before;

        #[derive(sqlx::FromRow)]
        struct EventRow {
            event_date: Date,
            description: String,
            reference: Option<String>,
            debit: i64,
            credit: i64,
        }

        let events: Vec<EventRow> = sqlx::query_as(
            "SELECT event_date, description, reference, debit, credit FROM (\
               SELECT inv.date AS event_date, \
                      'Invoice ' || inv.invoice_number AS description, \
                      inv.invoice_number AS reference, \
                      COALESCE(SUM(\
                        (il.quantity * il.unit_price / 100) + \
                        (il.quantity * il.unit_price / 100 * il.tax_rate / 10000)\
                      ), 0)::BIGINT AS debit, \
                      0::BIGINT AS credit \
               FROM invoices inv \
               JOIN invoice_lines il ON il.invoice_id = inv.id \
               WHERE inv.organization_id = $1 AND inv.contact_id = $2 \
                 AND inv.date BETWEEN $3 AND $4 \
                 AND inv.invoice_type = 'invoice' \
                 AND inv.status NOT IN ('draft', 'voided') \
               GROUP BY inv.id, inv.invoice_number, inv.date \
               UNION ALL \
               SELECT p.payment_date AS event_date, \
                      'Payment for ' || inv.invoice_number AS description, \
                      inv.invoice_number AS reference, \
                      0::BIGINT AS debit, \
                      p.amount AS credit \
               FROM payments p \
               JOIN invoices inv ON inv.id = p.invoice_id \
               WHERE inv.organization_id = $1 AND inv.contact_id = $2 \
                 AND p.payment_date BETWEEN $3 AND $4 \
                 AND inv.invoice_type = 'invoice'\
             ) sub ORDER BY event_date, description",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut running = opening_balance;
        let lines: Vec<StatementLine> = events
            .into_iter()
            .map(|e| {
                running += e.debit - e.credit;
                StatementLine {
                    date: e.event_date,
                    description: e.description,
                    reference: e.reference,
                    debit: e.debit,
                    credit: e.credit,
                    balance: running,
                }
            })
            .collect();

        Ok(ContactStatement {
            contact_id: contact_id.to_string(),
            from,
            to,
            opening_balance,
            closing_balance: running,
            lines,
        })
    }

    pub async fn consolidated_profit_loss(
        pool: &PgPool,
        org_ids: &[&str],
        from: Date,
        to: Date,
    ) -> Result<ConsolidatedProfitLoss, DbError> {
        let mut per_org: Vec<OrgProfitLoss> = Vec::with_capacity(org_ids.len());
        for org_id in org_ids {
            let report = Self::profit_loss(pool, org_id, from, to).await?;
            per_org.push(OrgProfitLoss {
                org_id: (*org_id).to_string(),
                report,
            });
        }
        let combined_revenue: i64 = per_org.iter().map(|o| o.report.revenue.total).sum();
        let combined_expenses: i64 = per_org.iter().map(|o| o.report.expenses.total).sum();
        let combined_net_income = combined_revenue - combined_expenses;
        Ok(ConsolidatedProfitLoss {
            from,
            to,
            per_org,
            combined_revenue,
            combined_expenses,
            combined_net_income,
        })
    }

    /// Returns all 1099-eligible vendor contacts with total payments made to them during `year`.
    pub async fn summary_1099(
        pool: &PgPool,
        org_id: &str,
        year: i32,
    ) -> Result<oxidebooks_core::models::Summary1099, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Uuid,
            contact_name: String,
            tax_id: Option<String>,
            total_paid: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT c.id AS contact_id, c.name AS contact_name, c.tax_id, \
             COALESCE(SUM(p.amount), 0)::BIGINT AS total_paid \
             FROM contacts c \
             LEFT JOIN invoices i ON i.contact_id = c.id \
               AND i.organization_id = c.organization_id \
               AND i.invoice_type = 'bill' \
             LEFT JOIN payments p ON p.invoice_id = i.id \
               AND EXTRACT(YEAR FROM p.payment_date) = $2 \
             WHERE c.organization_id = $1 AND c.is_1099_vendor = TRUE \
             GROUP BY c.id, c.name, c.tax_id \
             ORDER BY c.name ASC",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let vendors = rows
            .into_iter()
            .map(|r| oxidebooks_core::models::Vendor1099Row {
                contact_id: r.contact_id.to_string(),
                contact_name: r.contact_name,
                tax_id: r.tax_id,
                total_paid: r.total_paid,
            })
            .collect();

        Ok(oxidebooks_core::models::Summary1099 { year, vendors })
    }

    /// Global text search across contacts, invoices, products, and accounts.
    pub async fn search(
        pool: &PgPool,
        org_id: &str,
        query: &str,
        types: &[&str],
        limit: i64,
    ) -> Result<Vec<oxidebooks_core::models::SearchHit>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let pattern = format!("%{query}%");
        let mut hits: Vec<oxidebooks_core::models::SearchHit> = Vec::new();

        if types.contains(&"contacts") {
            #[derive(sqlx::FromRow)]
            struct ContactR {
                id: Uuid,
                name: String,
            }
            let rows: Vec<ContactR> = sqlx::query_as(
                "SELECT id, name FROM contacts \
                 WHERE organization_id = $1 AND name ILIKE $2 LIMIT $3",
            )
            .bind(org_uuid)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?;
            for r in rows {
                hits.push(oxidebooks_core::models::SearchHit {
                    id: r.id.to_string(),
                    display: r.name,
                    hit_type: "contact".into(),
                });
            }
        }

        if types.contains(&"invoices") {
            #[derive(sqlx::FromRow)]
            struct InvoiceR {
                id: Uuid,
                invoice_number: String,
            }
            let rows: Vec<InvoiceR> = sqlx::query_as(
                "SELECT id, invoice_number FROM invoices \
                 WHERE organization_id = $1 AND invoice_number ILIKE $2 LIMIT $3",
            )
            .bind(org_uuid)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?;
            for r in rows {
                hits.push(oxidebooks_core::models::SearchHit {
                    id: r.id.to_string(),
                    display: r.invoice_number,
                    hit_type: "invoice".into(),
                });
            }
        }

        if types.contains(&"products") {
            #[derive(sqlx::FromRow)]
            struct ProductR {
                id: Uuid,
                name: String,
            }
            let rows: Vec<ProductR> = sqlx::query_as(
                "SELECT id, name FROM products \
                 WHERE organization_id = $1 AND name ILIKE $2 LIMIT $3",
            )
            .bind(org_uuid)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?;
            for r in rows {
                hits.push(oxidebooks_core::models::SearchHit {
                    id: r.id.to_string(),
                    display: r.name,
                    hit_type: "product".into(),
                });
            }
        }

        if types.contains(&"accounts") {
            #[derive(sqlx::FromRow)]
            struct AccountR {
                id: Uuid,
                code: String,
                name: String,
            }
            let rows: Vec<AccountR> = sqlx::query_as(
                "SELECT id, code, name FROM accounts \
                 WHERE organization_id = $1 AND (name ILIKE $2 OR code ILIKE $2) LIMIT $3",
            )
            .bind(org_uuid)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?;
            for r in rows {
                hits.push(oxidebooks_core::models::SearchHit {
                    id: r.id.to_string(),
                    display: format!("{} – {}", r.code, r.name),
                    hit_type: "account".into(),
                });
            }
        }

        Ok(hits)
    }

    /// General ledger detail for a single account between two dates.
    pub async fn account_ledger(
        pool: &PgPool,
        org_id: &str,
        account_id: &str,
        from: Date,
        to: Date,
    ) -> Result<AccountLedger, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let acct_uuid = parse_uuid(account_id)?;

        // Account metadata.
        let meta: Option<(String, String, String)> = sqlx::query_as(
            "SELECT code, name, account_type FROM accounts \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(acct_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let (code, name, acct_type_str) = meta.ok_or(DbError::NotFound)?;
        let account_type = AccountType::from_str(&acct_type_str).unwrap_or(AccountType::Asset);

        // Opening balance: sum of journal lines before `from`.
        let opening: (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(jl.debit), 0)::BIGINT, COALESCE(SUM(jl.credit), 0)::BIGINT \
             FROM journal_lines jl \
             JOIN journal_entries je ON je.id = jl.journal_entry_id \
             WHERE jl.account_id = $1 AND je.organization_id = $2 \
               AND je.status = 'posted' AND je.date < $3",
        )
        .bind(acct_uuid)
        .bind(org_uuid)
        .bind(from)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let opening_balance = if account_type.is_debit_normal() {
            opening.0 - opening.1
        } else {
            opening.1 - opening.0
        };

        // Lines within the date range.
        #[derive(sqlx::FromRow)]
        struct RawLine {
            journal_entry_id: Uuid,
            date: Date,
            description: String,
            reference: Option<String>,
            debit: i64,
            credit: i64,
        }

        let raw: Vec<RawLine> = sqlx::query_as(
            "SELECT je.id AS journal_entry_id, je.date, je.description, je.reference, \
                    jl.debit, jl.credit \
             FROM journal_lines jl \
             JOIN journal_entries je ON je.id = jl.journal_entry_id \
             WHERE jl.account_id = $1 AND je.organization_id = $2 \
               AND je.status = 'posted' AND je.date >= $3 AND je.date <= $4 \
             ORDER BY je.date ASC, je.id ASC",
        )
        .bind(acct_uuid)
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut running = opening_balance;
        let lines: Vec<LedgerLine> = raw
            .into_iter()
            .map(|r| {
                let movement = if account_type.is_debit_normal() {
                    r.debit - r.credit
                } else {
                    r.credit - r.debit
                };
                running += movement;
                LedgerLine {
                    journal_entry_id: r.journal_entry_id.to_string(),
                    date: r.date,
                    description: r.description,
                    reference: r.reference,
                    debit: r.debit,
                    credit: r.credit,
                    running_balance: running,
                }
            })
            .collect();

        let closing_balance = running;

        Ok(AccountLedger {
            account_id: acct_uuid.to_string(),
            account_code: code,
            account_name: name,
            from,
            to,
            opening_balance,
            lines,
            closing_balance,
        })
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct SalesByProductRowDb {
    product_id: Uuid,
    product_name: String,
    sku: Option<String>,
    quantity: i64,
    gross_amount: i64,
    discount_amount: i64,
    net_amount: i64,
    tax_amount: i64,
}

impl ReportRepo {
    pub async fn sales_by_product(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<SalesByProductReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<SalesByProductRowDb> = sqlx::query_as(
            r#"
            SELECT
                p.id                                                           AS product_id,
                p.name                                                         AS product_name,
                p.sku,
                SUM(il.quantity)                                               AS quantity,
                SUM(il.quantity * il.unit_price / 100)                        AS gross_amount,
                SUM(il.quantity * il.unit_price / 100 * il.discount_pct / 10000) AS discount_amount,
                SUM(il.quantity * il.unit_price / 100
                    - il.quantity * il.unit_price / 100 * il.discount_pct / 10000)
                                                                               AS net_amount,
                SUM((il.quantity * il.unit_price / 100
                     - il.quantity * il.unit_price / 100 * il.discount_pct / 10000)
                    * il.tax_rate / 10000)                                     AS tax_amount
            FROM invoice_lines il
            JOIN invoices i  ON i.id  = il.invoice_id
            JOIN products  p ON p.id  = il.product_id
            WHERE i.organization_id = $1
              AND i.invoice_type     = 'invoice'
              AND i.status NOT IN ('voided', 'draft')
              AND i.date >= $2
              AND i.date <= $3
              AND il.product_id IS NOT NULL
            GROUP BY p.id, p.name, p.sku
            ORDER BY net_amount DESC
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total_gross: i64 = rows.iter().map(|r| r.gross_amount).sum();
        let total_discount: i64 = rows.iter().map(|r| r.discount_amount).sum();
        let total_net: i64 = rows.iter().map(|r| r.net_amount).sum();
        let total_tax: i64 = rows.iter().map(|r| r.tax_amount).sum();

        let result_rows = rows
            .into_iter()
            .map(|r| SalesByProductRow {
                product_id: r.product_id.to_string(),
                product_name: r.product_name,
                sku: r.sku,
                quantity: r.quantity,
                gross_amount: r.gross_amount,
                discount_amount: r.discount_amount,
                net_amount: r.net_amount,
                tax_amount: r.tax_amount,
            })
            .collect();

        Ok(SalesByProductReport {
            from,
            to,
            rows: result_rows,
            total_gross,
            total_discount,
            total_net,
            total_tax,
        })
    }

    pub async fn project_profitability(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<ProjectProfitabilityReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            project_id: Uuid,
            project_name: String,
            invoiced_amount: i64,
            expense_amount: i64,
            time_cost: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                p.id                                                   AS project_id,
                p.name                                                 AS project_name,
                COALESCE(inv.invoiced_amount, 0)                       AS invoiced_amount,
                COALESCE(exp.expense_amount, 0)                        AS expense_amount,
                COALESCE(te.time_cost, 0)                              AS time_cost
            FROM projects p
            LEFT JOIN LATERAL (
                SELECT COALESCE(SUM(
                    il.quantity * il.unit_price / 100
                    - il.quantity * il.unit_price / 100 * il.discount_pct / 10000
                ), 0)::BIGINT AS invoiced_amount
                FROM invoices i
                JOIN invoice_lines il ON il.invoice_id = i.id
                WHERE i.organization_id = $1
                  AND i.project_id = p.id
                  AND i.invoice_type = 'invoice'
                  AND i.status NOT IN ('draft', 'voided')
            ) inv ON TRUE
            LEFT JOIN LATERAL (
                SELECT COALESCE(SUM(e.amount), 0)::BIGINT AS expense_amount
                FROM expenses e
                WHERE e.organization_id = $1
                  AND e.project_id = p.id
                  AND e.status NOT IN ('draft', 'rejected')
            ) exp ON TRUE
            LEFT JOIN LATERAL (
                SELECT COALESCE(SUM(te.duration_minutes * te.hourly_rate / 60), 0)::BIGINT AS time_cost
                FROM time_entries te
                WHERE te.organization_id = $1
                  AND te.project_id = p.id
                  AND te.billed = TRUE
            ) te ON TRUE
            WHERE p.organization_id = $1
              AND p.status = 'active'
            ORDER BY (COALESCE(inv.invoiced_amount, 0) - COALESCE(exp.expense_amount, 0) - COALESCE(te.time_cost, 0)) DESC
            "#,
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut result_rows = Vec::with_capacity(rows.len());
        let mut total_invoiced = 0i64;
        let mut total_expenses = 0i64;
        let mut total_time = 0i64;

        for r in rows {
            let gross_profit = r.invoiced_amount - r.expense_amount - r.time_cost;
            let margin_bps = if r.invoiced_amount > 0 {
                gross_profit * 10_000 / r.invoiced_amount
            } else {
                0
            };
            total_invoiced += r.invoiced_amount;
            total_expenses += r.expense_amount;
            total_time += r.time_cost;
            result_rows.push(ProjectProfitabilityRow {
                project_id: r.project_id.to_string(),
                project_name: r.project_name,
                invoiced_amount: r.invoiced_amount,
                expense_amount: r.expense_amount,
                time_cost: r.time_cost,
                gross_profit,
                margin_bps,
            });
        }

        let total_profit = total_invoiced - total_expenses - total_time;

        Ok(ProjectProfitabilityReport {
            rows: result_rows,
            total_invoiced,
            total_expenses,
            total_time_cost: total_time,
            total_profit,
        })
    }

    /// Cash flow forecast: project AR inflows and AP outflows by weekly bucket
    /// over the next `days` days, starting from `from_date`.
    pub async fn cash_flow_forecast(
        pool: &PgPool,
        org_id: &str,
        from_date: Date,
        days: i64,
    ) -> Result<CashFlowForecast, DbError> {
        let org = parse_uuid(org_id)?;
        let to_date = from_date + time::Duration::days(days);

        // Opening cash balance: sum of cash/bank accounts
        let opening_balance: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(jl.debit - jl.credit), 0)::BIGINT
             FROM journal_lines jl
             JOIN accounts a ON a.id = jl.account_id
             WHERE a.organization_id = $1 AND a.sub_type IN ('bank','cash')",
        )
        .bind(org)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Open AR by due_date (inflows)
        let ar_rows: Vec<(Date, i64)> = sqlx::query_as(
            "SELECT due_date, SUM(total_amount - paid_amount)::BIGINT
             FROM invoices
             WHERE organization_id = $1
               AND invoice_type = 'invoice'
               AND status NOT IN ('voided','draft','paid')
               AND due_date BETWEEN $2 AND $3
             GROUP BY due_date ORDER BY due_date",
        )
        .bind(org)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Open AP by due_date (outflows)
        let ap_rows: Vec<(Date, i64)> = sqlx::query_as(
            "SELECT due_date, SUM(total_amount - paid_amount)::BIGINT
             FROM invoices
             WHERE organization_id = $1
               AND invoice_type = 'bill'
               AND status NOT IN ('voided','draft','paid')
               AND due_date BETWEEN $2 AND $3
             GROUP BY due_date ORDER BY due_date",
        )
        .bind(org)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Build weekly buckets
        let week_count = (days / 7).max(1) as usize;
        let mut buckets: Vec<CashFlowForecastBucket> = (0..week_count)
            .map(|i| {
                let start = from_date + time::Duration::days(i as i64 * 7);
                let end = (start + time::Duration::days(6)).min(to_date);
                CashFlowForecastBucket {
                    period_start: start,
                    period_end: end,
                    inflows: 0,
                    outflows: 0,
                    net: 0,
                    running_balance: 0,
                }
            })
            .collect();

        let bucket_for = |d: Date| -> usize {
            let diff = (d - from_date).whole_days().max(0) as usize;
            (diff / 7).min(week_count - 1)
        };

        for (due, amount) in ar_rows {
            buckets[bucket_for(due)].inflows += amount;
        }
        for (due, amount) in ap_rows {
            buckets[bucket_for(due)].outflows += amount;
        }

        let mut running = opening_balance;
        for b in &mut buckets {
            b.net = b.inflows - b.outflows;
            running += b.net;
            b.running_balance = running;
        }

        Ok(CashFlowForecast {
            opening_balance,
            buckets,
            closing_balance: running,
        })
    }

    /// Job costing report — actual time/expense/bill costs vs project budget,
    /// broken down by cost code.
    pub async fn job_costing(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
        project_id: Option<&str>,
    ) -> Result<JobCostingReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let proj_uuid: Option<Uuid> = project_id.map(parse_uuid).transpose()?;

        #[derive(sqlx::FromRow)]
        struct ProjRow {
            project_id: Uuid,
            project_name: String,
            budget: i64,
        }

        let projects: Vec<ProjRow> = if let Some(pid) = proj_uuid {
            sqlx::query_as(
                "SELECT id AS project_id, name AS project_name, \
                 COALESCE(budget, 0) AS budget \
                 FROM projects WHERE organization_id = $1 AND id = $2",
            )
            .bind(org_uuid)
            .bind(pid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id AS project_id, name AS project_name, \
                 COALESCE(budget, 0) AS budget \
                 FROM projects WHERE organization_id = $1 ORDER BY name ASC",
            )
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        #[derive(sqlx::FromRow)]
        struct TimeCostRow {
            project_id: Uuid,
            cost_code_id: Option<Uuid>,
            cost_code: Option<String>,
            cost_code_name: Option<String>,
            cost_type: Option<String>,
            total: i64,
        }

        // Time cost = hours * hourly_rate from time_entries.
        let time_costs: Vec<TimeCostRow> = sqlx::query_as(
            "SELECT te.project_id, te.cost_code_id, \
                    cc.code AS cost_code, cc.name AS cost_code_name, cc.cost_type, \
                    COALESCE(SUM(te.duration_minutes * te.hourly_rate / 60), 0)::BIGINT AS total \
             FROM time_entries te \
             LEFT JOIN cost_codes cc ON cc.id = te.cost_code_id \
             WHERE te.organization_id = $1 \
               AND te.date BETWEEN $2 AND $3 \
               AND te.project_id IS NOT NULL \
             GROUP BY te.project_id, te.cost_code_id, cc.code, cc.name, cc.cost_type",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Expense cost per project/cost_code.
        let expense_costs: Vec<TimeCostRow> = sqlx::query_as(
            "SELECT e.project_id, e.cost_code_id, \
                    cc.code AS cost_code, cc.name AS cost_code_name, cc.cost_type, \
                    COALESCE(SUM(e.amount), 0)::BIGINT AS total \
             FROM expenses e \
             LEFT JOIN cost_codes cc ON cc.id = e.cost_code_id \
             WHERE e.organization_id = $1 \
               AND e.expense_date BETWEEN $2 AND $3 \
               AND e.project_id IS NOT NULL \
             GROUP BY e.project_id, e.cost_code_id, cc.code, cc.name, cc.cost_type",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut rows: Vec<JobCostingRow> = projects
            .into_iter()
            .map(|p| {
                let tc: i64 = time_costs
                    .iter()
                    .filter(|r| r.project_id == p.project_id)
                    .map(|r| r.total)
                    .sum();
                let ec: i64 = expense_costs
                    .iter()
                    .filter(|r| r.project_id == p.project_id)
                    .map(|r| r.total)
                    .sum();
                let total_actual = tc + ec;
                let variance = p.budget - total_actual;

                let mut cc_map: std::collections::HashMap<Option<Uuid>, JobCostingCostCodeRow> =
                    std::collections::HashMap::new();

                for r in time_costs
                    .iter()
                    .filter(|r| r.project_id == p.project_id)
                    .chain(
                        expense_costs
                            .iter()
                            .filter(|r| r.project_id == p.project_id),
                    )
                {
                    let entry =
                        cc_map
                            .entry(r.cost_code_id)
                            .or_insert_with(|| JobCostingCostCodeRow {
                                cost_code_id: r
                                    .cost_code_id
                                    .map(|u| u.to_string())
                                    .unwrap_or_default(),
                                cost_code: r.cost_code.clone().unwrap_or_else(|| "uncoded".into()),
                                cost_code_name: r
                                    .cost_code_name
                                    .clone()
                                    .unwrap_or_else(|| "Uncoded".into()),
                                cost_type: r.cost_type.clone().unwrap_or_else(|| "other".into()),
                                actual_cost: 0,
                            });
                    entry.actual_cost += r.total;
                }

                let mut cost_codes: Vec<JobCostingCostCodeRow> = cc_map.into_values().collect();
                cost_codes.sort_by(|a, b| a.cost_code.cmp(&b.cost_code));

                JobCostingRow {
                    project_id: p.project_id.to_string(),
                    project_name: p.project_name,
                    budget: p.budget,
                    time_cost: tc,
                    expense_cost: ec,
                    bill_cost: 0,
                    total_actual,
                    variance,
                    cost_codes,
                }
            })
            .collect();

        rows.retain(|r| r.total_actual > 0 || r.budget > 0);

        let total_budget: i64 = rows.iter().map(|r| r.budget).sum();
        let total_actual: i64 = rows.iter().map(|r| r.total_actual).sum();

        Ok(JobCostingReport {
            from,
            to,
            rows,
            total_budget,
            total_actual,
            total_variance: total_budget - total_actual,
        })
    }

    /// Vendor spend report — bills by vendor aggregated over a period.
    pub async fn vendor_spend(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<VendorSpendReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            contact_id: Option<Uuid>,
            vendor_name: String,
            bill_count: i64,
            subtotal: i64,
            amount_paid: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                vb.contact_id,
                COALESCE(c.name, 'No Vendor') AS vendor_name,
                COUNT(vb.id)::BIGINT           AS bill_count,
                COALESCE(SUM(
                    (SELECT COALESCE(SUM(quantity * unit_price), 0)
                     FROM bill_lines WHERE bill_id = vb.id)
                ), 0)::BIGINT                  AS subtotal,
                COALESCE(SUM(
                    (SELECT COALESCE(SUM(amount), 0)
                     FROM bill_payments WHERE bill_id = vb.id)
                ), 0)::BIGINT                  AS amount_paid
            FROM vendor_bills vb
            LEFT JOIN contacts c ON c.id = vb.contact_id
            WHERE vb.organization_id = $1
              AND vb.bill_date BETWEEN $2 AND $3
              AND vb.status NOT IN ('voided')
            GROUP BY vb.contact_id, c.name
            ORDER BY subtotal DESC
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let spend_rows: Vec<VendorSpendRow> = rows
            .iter()
            .map(|r| {
                let outstanding = (r.subtotal - r.amount_paid).max(0);
                VendorSpendRow {
                    contact_id: r.contact_id.map(|u| u.to_string()).unwrap_or_default(),
                    vendor_name: r.vendor_name.clone(),
                    bill_count: r.bill_count,
                    subtotal: r.subtotal,
                    tax_amount: 0,
                    total_paid: r.amount_paid,
                    outstanding,
                }
            })
            .collect();

        let total_bills: i64 = spend_rows.iter().map(|r| r.bill_count).sum();
        let total_subtotal: i64 = spend_rows.iter().map(|r| r.subtotal).sum();
        let total_paid: i64 = spend_rows.iter().map(|r| r.total_paid).sum();
        let total_outstanding: i64 = spend_rows.iter().map(|r| r.outstanding).sum();

        Ok(VendorSpendReport {
            from,
            to,
            rows: spend_rows,
            total_bills,
            total_subtotal,
            total_paid,
            total_outstanding,
        })
    }

    pub async fn payroll_summary(
        pool: &PgPool,
        org_id: &str,
        from: Date,
        to: Date,
    ) -> Result<PayrollSummaryReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            run_id: Uuid,
            period_start: time::Date,
            period_end: time::Date,
            status: String,
            employee_count: i64,
            total_gross: i64,
            total_tax: i64,
            total_deductions: i64,
            total_net: i64,
        }

        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT
               pr.id          AS run_id,
               pr.period_start,
               pr.period_end,
               pr.status,
               COUNT(pe.id)                                  AS employee_count,
               COALESCE(SUM(pe.gross_pay), 0)::BIGINT        AS total_gross,
               COALESCE(SUM(pe.tax_withheld), 0)::BIGINT     AS total_tax,
               COALESCE(SUM(pe.other_deductions), 0)::BIGINT AS total_deductions,
               COALESCE(SUM(pe.net_pay), 0)::BIGINT          AS total_net
             FROM payroll_runs pr
             LEFT JOIN payroll_entries pe ON pe.payroll_run_id = pr.id
             WHERE pr.organization_id = $1
               AND pr.period_start >= $2
               AND pr.period_end   <= $3
             GROUP BY pr.id, pr.period_start, pr.period_end, pr.status
             ORDER BY pr.period_start DESC",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let total_gross: i64 = rows.iter().map(|r| r.total_gross).sum();
        let total_tax: i64 = rows.iter().map(|r| r.total_tax).sum();
        let total_net: i64 = rows.iter().map(|r| r.total_net).sum();

        let summary_rows: Vec<PayrollSummaryRow> = rows
            .into_iter()
            .map(|r| PayrollSummaryRow {
                run_id: r.run_id.to_string(),
                period_start: r.period_start,
                period_end: r.period_end,
                status: r.status,
                employee_count: r.employee_count,
                total_gross: r.total_gross,
                total_tax: r.total_tax,
                total_deductions: r.total_deductions,
                total_net: r.total_net,
            })
            .collect();

        Ok(PayrollSummaryReport {
            from,
            to,
            rows: summary_rows,
            total_gross,
            total_tax,
            total_net,
        })
    }

    /// Per-employee annual wages and withholding for W-2 preparation.
    /// Only includes paid payroll runs.
    pub async fn w2_data(pool: &PgPool, org_id: &str, year: i32) -> Result<Vec<W2Row>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            user_id: Uuid,
            employee_name: String,
            email: String,
            wages: i64,
            federal_income_tax_withheld: i64,
            other_deductions: i64,
            net_pay: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT
               u.id                                           AS user_id,
               u.name                                        AS employee_name,
               u.email,
               COALESCE(SUM(pe.gross_pay), 0)::BIGINT        AS wages,
               COALESCE(SUM(pe.tax_withheld), 0)::BIGINT     AS federal_income_tax_withheld,
               COALESCE(SUM(pe.other_deductions), 0)::BIGINT AS other_deductions,
               COALESCE(SUM(pe.net_pay), 0)::BIGINT          AS net_pay
             FROM payroll_entries pe
             JOIN payroll_runs pr ON pr.id = pe.payroll_run_id
             JOIN users u ON u.id = pe.user_id
             WHERE pr.organization_id = $1
               AND EXTRACT(YEAR FROM pr.period_start) = $2
               AND pr.status = 'paid'
             GROUP BY u.id, u.name, u.email
             ORDER BY u.name",
        )
        .bind(org_uuid)
        .bind(year)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| W2Row {
                user_id: r.user_id.to_string(),
                employee_name: r.employee_name,
                email: r.email,
                year,
                wages: r.wages,
                federal_income_tax_withheld: r.federal_income_tax_withheld,
                other_deductions: r.other_deductions,
                net_pay: r.net_pay,
            })
            .collect())
    }

    /// Quarterly payroll tax aggregates for Form 941 preparation.
    pub async fn form_941_data(
        pool: &PgPool,
        org_id: &str,
        year: i32,
        quarter: i32,
    ) -> Result<Form941Quarter, DbError> {
        if !(1..=4).contains(&quarter) {
            return Err(DbError::Conflict("quarter must be 1-4".into()));
        }
        let org_uuid = parse_uuid(org_id)?;

        // Employee count and total wages from payroll entries.
        let (employee_count, wages): (i64, i64) = sqlx::query_as(
            "SELECT
               COUNT(DISTINCT pe.user_id)::BIGINT,
               COALESCE(SUM(pe.gross_pay), 0)::BIGINT
             FROM payroll_entries pe
             JOIN payroll_runs pr ON pr.id = pe.payroll_run_id
             WHERE pr.organization_id = $1
               AND EXTRACT(YEAR FROM pr.period_start) = $2
               AND EXTRACT(QUARTER FROM pr.period_start) = $3
               AND pr.status = 'paid'",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(quarter)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Per-type tax amounts from payroll_tax_liabilities.
        #[derive(sqlx::FromRow)]
        struct TaxRow {
            tax_type: String,
            employee_amount: i64,
            employer_amount: i64,
        }

        let tax_rows: Vec<TaxRow> = sqlx::query_as(
            "SELECT
               ptl.tax_type,
               COALESCE(SUM(ptl.employee_amount), 0)::BIGINT AS employee_amount,
               COALESCE(SUM(ptl.employer_amount), 0)::BIGINT AS employer_amount
             FROM payroll_tax_liabilities ptl
             JOIN payroll_runs pr ON pr.id = ptl.payroll_run_id
             WHERE ptl.organization_id = $1
               AND EXTRACT(YEAR FROM pr.period_start) = $2
               AND EXTRACT(QUARTER FROM pr.period_start) = $3
             GROUP BY ptl.tax_type",
        )
        .bind(org_uuid)
        .bind(year)
        .bind(quarter)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut federal_income_tax = 0i64;
        let mut social_security_employee = 0i64;
        let mut social_security_employer = 0i64;
        let mut medicare_employee = 0i64;
        let mut medicare_employer = 0i64;
        let mut total_deposits = 0i64;

        for row in &tax_rows {
            match row.tax_type.as_str() {
                "federal_income" => federal_income_tax += row.employee_amount,
                "social_security" => {
                    social_security_employee += row.employee_amount;
                    social_security_employer += row.employer_amount;
                }
                "medicare" => {
                    medicare_employee += row.employee_amount;
                    medicare_employer += row.employer_amount;
                }
                _ => {}
            }
            total_deposits += row.employee_amount + row.employer_amount;
        }

        Ok(Form941Quarter {
            year,
            quarter,
            employee_count,
            wages,
            federal_income_tax,
            social_security_employee,
            social_security_employer,
            medicare_employee,
            medicare_employer,
            total_deposits,
        })
    }

    /// Side-by-side P&L comparison between two periods.
    pub async fn pl_comparison(
        pool: &PgPool,
        org_id: &str,
        current_from: Date,
        current_to: Date,
        prior_from: Date,
        prior_to: Date,
    ) -> Result<PLComparisonReport, DbError> {
        let current = Self::profit_loss(pool, org_id, current_from, current_to).await?;
        let prior = Self::profit_loss(pool, org_id, prior_from, prior_to).await?;
        let net_income_change = current.net_income - prior.net_income;
        let net_income_change_bps = if prior.net_income == 0 {
            0
        } else {
            net_income_change * 10_000 / prior.net_income
        };
        Ok(PLComparisonReport {
            current,
            prior,
            net_income_change,
            net_income_change_bps,
        })
    }

    /// GRNI (Goods Received Not Invoiced) accrual report.
    ///
    /// Returns GRN lines for purchase orders that have been received but not yet
    /// fully invoiced (i.e., no approved/paid vendor bill linked to the PO).
    pub async fn grni_accrual(
        pool: &PgPool,
        org_id: &str,
        as_of: Date,
    ) -> Result<GrniReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct GrniRowDb {
            po_id: Uuid,
            po_number: String,
            contact_id: Uuid,
            contact_name: String,
            grn_id: Uuid,
            receipt_date: Date,
            product_description: String,
            quantity_received: i64,
            unit_cost: i64,
        }

        let rows: Vec<GrniRowDb> = sqlx::query_as(
            r#"
            SELECT
                po.id               AS po_id,
                po.po_number        AS po_number,
                c.id                AS contact_id,
                c.name              AS contact_name,
                g.id                AS grn_id,
                g.receipt_date      AS receipt_date,
                COALESCE(gl.description, p.name, 'Unknown') AS product_description,
                gl.quantity_received,
                COALESCE(pol.unit_price, 0)::BIGINT AS unit_cost
            FROM goods_receipt_notes g
            JOIN purchase_orders po ON po.id = g.purchase_order_id
            JOIN contacts c ON c.id = po.contact_id
            JOIN grn_lines gl ON gl.grn_id = g.id
            LEFT JOIN purchase_order_lines pol ON pol.id = gl.po_line_id
            LEFT JOIN products p ON p.id = pol.product_id
            WHERE po.organization_id = $1
              AND g.receipt_date <= $2
              AND gl.quantity_received > 0
              AND NOT EXISTS (
                  SELECT 1 FROM vendor_bills vb
                  WHERE vb.purchase_order_id = po.id
                    AND vb.status IN ('approved', 'partial', 'paid')
              )
            ORDER BY g.receipt_date ASC, po.po_number ASC
            "#,
        )
        .bind(org_uuid)
        .bind(as_of)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let result_rows: Vec<GrniRow> = rows
            .iter()
            .map(|r| {
                let accrual_amount = r.quantity_received * r.unit_cost;
                GrniRow {
                    po_id: r.po_id.to_string(),
                    po_number: r.po_number.clone(),
                    contact_id: r.contact_id.to_string(),
                    contact_name: r.contact_name.clone(),
                    grn_id: r.grn_id.to_string(),
                    receipt_date: r.receipt_date,
                    product_description: r.product_description.clone(),
                    quantity_received: r.quantity_received,
                    unit_cost: r.unit_cost,
                    accrual_amount,
                }
            })
            .collect();

        let total_accrual = result_rows.iter().map(|r| r.accrual_amount).sum();

        Ok(GrniReport {
            as_of,
            rows: result_rows,
            total_accrual,
        })
    }
}
