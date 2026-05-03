use oxidebooks_core::models::{
    AccountBalance, AccountType, BalanceSheetReport, ProfitLossReport, ReportLine, ReportSection,
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
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
