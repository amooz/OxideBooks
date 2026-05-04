use oxidebooks_core::models::{BankReconciliationStatement, CreateBankReconciliationStatement};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ReconciliationStatementRow {
    id: Uuid,
    organization_id: Uuid,
    bank_account_id: Uuid,
    statement_date: Date,
    statement_balance: i64,
    book_balance: i64,
    outstanding_deposits: i64,
    outstanding_checks: i64,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

impl From<ReconciliationStatementRow> for BankReconciliationStatement {
    fn from(r: ReconciliationStatementRow) -> Self {
        BankReconciliationStatement {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            bank_account_id: r.bank_account_id.to_string(),
            statement_date: r.statement_date,
            statement_balance: r.statement_balance,
            book_balance: r.book_balance,
            outstanding_deposits: r.outstanding_deposits,
            outstanding_checks: r.outstanding_checks,
            notes: r.notes,
            created_at: r.created_at,
        }
    }
}

const COLS: &str = "id, organization_id, bank_account_id, statement_date, statement_balance, \
    book_balance, outstanding_deposits, outstanding_checks, notes, created_at";

pub struct BankReconciliationRepo;

impl BankReconciliationRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        bank_account_id: Option<&str>,
    ) -> Result<Vec<BankReconciliationStatement>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ReconciliationStatementRow> = if let Some(ba_id) = bank_account_id {
            let ba_uuid = parse_uuid(ba_id)?;
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM bank_reconciliation_statements \
                 WHERE organization_id = $1 AND bank_account_id = $2 \
                 ORDER BY statement_date DESC"
            ))
            .bind(org_uuid)
            .bind(ba_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM bank_reconciliation_statements \
                 WHERE organization_id = $1 ORDER BY statement_date DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        Ok(rows
            .into_iter()
            .map(BankReconciliationStatement::from)
            .collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<BankReconciliationStatement, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: Option<ReconciliationStatementRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM bank_reconciliation_statements \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(BankReconciliationStatement::from)
            .ok_or(DbError::NotFound)
    }

    /// Create a reconciliation statement, computing book_balance and outstanding items
    /// from the bank transactions table.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateBankReconciliationStatement,
    ) -> Result<BankReconciliationStatement, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ba_uuid = parse_uuid(&input.bank_account_id)?;

        // Outstanding deposits: unmatched/uncleared positive transactions on or before statement date.
        let outstanding_deposits: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT \
             FROM bank_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 \
               AND amount > 0 AND status = 'unmatched' \
               AND transaction_date <= $3",
        )
        .bind(org_uuid)
        .bind(ba_uuid)
        .bind(input.statement_date)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Outstanding checks: unmatched negative transactions (payments not yet cleared).
        let outstanding_checks: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(ABS(amount)), 0)::BIGINT \
             FROM bank_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 \
               AND amount < 0 AND status = 'unmatched' \
               AND transaction_date <= $3",
        )
        .bind(org_uuid)
        .bind(ba_uuid)
        .bind(input.statement_date)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Book balance: sum of all matched transactions up to statement date.
        let book_balance: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT \
             FROM bank_transactions \
             WHERE organization_id = $1 AND bank_account_id = $2 \
               AND status = 'matched' AND transaction_date <= $3",
        )
        .bind(org_uuid)
        .bind(ba_uuid)
        .bind(input.statement_date)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO bank_reconciliation_statements \
             (id, organization_id, bank_account_id, statement_date, statement_balance, \
              book_balance, outstanding_deposits, outstanding_checks, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(ba_uuid)
        .bind(input.statement_date)
        .bind(input.statement_balance)
        .bind(book_balance.0)
        .bind(outstanding_deposits.0)
        .bind(outstanding_checks.0)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: ReconciliationStatementRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM bank_reconciliation_statements WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "DELETE FROM bank_reconciliation_statements \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            Err(DbError::NotFound)
        } else {
            Ok(())
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
