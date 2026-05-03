use oxidebooks_core::models::{
    Budget, BudgetLine, BudgetVsActualLine, BudgetVsActualReport, CreateBudget, UpdateBudget,
    UpsertBudgetLine,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct BudgetRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    fiscal_year: i32,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct BudgetLineRow {
    id: Uuid,
    budget_id: Uuid,
    account_id: Uuid,
    month: i32,
    amount: i64,
}

fn budget_from_row(r: BudgetRow) -> Budget {
    Budget {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        fiscal_year: r.fiscal_year,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct BudgetRepo;

impl BudgetRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Budget>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<BudgetRow> = sqlx::query_as(
            "SELECT id, organization_id, name, fiscal_year, is_active, created_at, updated_at \
             FROM budgets WHERE organization_id = $1 ORDER BY fiscal_year DESC, name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(budget_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Budget, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: BudgetRow = sqlx::query_as(
            "SELECT id, organization_id, name, fiscal_year, is_active, created_at, updated_at \
             FROM budgets WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(budget_from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateBudget,
    ) -> Result<Budget, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO budgets (organization_id, name, fiscal_year) \
             VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.fiscal_year)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateBudget,
    ) -> Result<Budget, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE budgets SET \
             name      = COALESCE($1, name), \
             is_active = COALESCE($2, is_active), \
             updated_at = NOW() \
             WHERE id = $3 AND organization_id = $4",
        )
        .bind(input.name)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM budgets WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn list_lines(pool: &PgPool, budget_id: &str) -> Result<Vec<BudgetLine>, DbError> {
        let bid = parse_uuid(budget_id)?;
        let rows: Vec<BudgetLineRow> = sqlx::query_as(
            "SELECT id, budget_id, account_id, month, amount FROM budget_lines \
             WHERE budget_id = $1 ORDER BY month, account_id",
        )
        .bind(bid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| BudgetLine {
                id: r.id.to_string(),
                budget_id: r.budget_id.to_string(),
                account_id: r.account_id.to_string(),
                month: r.month,
                amount: r.amount,
            })
            .collect())
    }

    /// Batch upsert budget lines (insert or update by budget_id+account_id+month).
    pub async fn upsert_lines(
        pool: &PgPool,
        budget_id: &str,
        lines: Vec<UpsertBudgetLine>,
    ) -> Result<Vec<BudgetLine>, DbError> {
        let bid = parse_uuid(budget_id)?;
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        for line in lines {
            if line.month < 1 || line.month > 12 {
                return Err(DbError::Conflict(format!(
                    "month {} out of range 1–12",
                    line.month
                )));
            }
            let acct_uuid = parse_uuid(&line.account_id)?;
            sqlx::query(
                "INSERT INTO budget_lines (budget_id, account_id, month, amount) \
                 VALUES ($1,$2,$3,$4) \
                 ON CONFLICT (budget_id, account_id, month) \
                 DO UPDATE SET amount = EXCLUDED.amount",
            )
            .bind(bid)
            .bind(acct_uuid)
            .bind(line.month)
            .bind(line.amount)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::list_lines(pool, budget_id).await
    }

    /// Budget vs actual: budgeted amounts vs posted journal entries in the same period.
    pub async fn budget_vs_actual(
        pool: &PgPool,
        org_id: &str,
        budget_id: &str,
        from: Date,
        to: Date,
    ) -> Result<BudgetVsActualReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let bid = parse_uuid(budget_id)?;

        let budget = Self::get_by_id(pool, org_id, budget_id).await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: Uuid,
            account_code: String,
            account_name: String,
            month: i32,
            budgeted: i64,
            actual: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                a.id          AS account_id,
                a.code        AS account_code,
                a.name        AS account_name,
                bl.month,
                bl.amount     AS budgeted,
                COALESCE((
                    SELECT SUM(jl.debit - jl.credit)
                    FROM journal_lines jl
                    JOIN journal_entries je ON je.id = jl.journal_entry_id
                    WHERE jl.account_id = a.id
                      AND je.organization_id = $1
                      AND je.status = 'posted'
                      AND EXTRACT(YEAR  FROM je.date) = $4
                      AND EXTRACT(MONTH FROM je.date) = bl.month
                      AND je.date BETWEEN $2 AND $3
                ), 0)::BIGINT AS actual
            FROM budget_lines bl
            JOIN accounts a ON a.id = bl.account_id
            WHERE bl.budget_id = $5
            ORDER BY bl.month, a.code
            "#,
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .bind(budget.fiscal_year)
        .bind(bid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let lines: Vec<BudgetVsActualLine> = rows
            .iter()
            .map(|r| {
                let variance = r.budgeted - r.actual;
                BudgetVsActualLine {
                    account_id: r.account_id.to_string(),
                    account_code: r.account_code.clone(),
                    account_name: r.account_name.clone(),
                    month: r.month,
                    budgeted: r.budgeted,
                    actual: r.actual,
                    variance,
                }
            })
            .collect();

        let total_budgeted = lines.iter().map(|l| l.budgeted).sum();
        let total_actual = lines.iter().map(|l| l.actual).sum();

        Ok(BudgetVsActualReport {
            budget_id: budget_id.to_string(),
            budget_name: budget.name,
            fiscal_year: budget.fiscal_year,
            lines,
            total_budgeted,
            total_actual,
            total_variance: total_budgeted - total_actual,
        })
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
