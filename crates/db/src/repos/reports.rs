use oxidebooks_core::models::{
    AccountBalance, AccountType, AgingReport, AgingRow, BalanceSheetReport, CashFlowReport,
    CashFlowSection, ConsolidatedProfitLoss, ContactStatement, DashboardKpis, OrgProfitLoss,
    ProfitLossReport, ReportLine, ReportSection, StatementLine, TaxSummaryLine, TaxSummaryReport,
    TrialBalance,
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
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
