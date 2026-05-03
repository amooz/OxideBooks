use oxidebooks_core::models::{AccountBalance, AccountType, TrialBalance};
use sqlx::PgPool;
use std::str::FromStr;
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
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
