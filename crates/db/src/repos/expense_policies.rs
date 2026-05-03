use oxidebooks_core::models::{ExpensePolicy, UpsertExpensePolicy};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PolicyRow {
    id: Uuid,
    organization_id: Uuid,
    category: String,
    max_amount: i64,
    requires_receipt_above: i64,
    created_at: OffsetDateTime,
}

fn from_row(r: PolicyRow) -> ExpensePolicy {
    ExpensePolicy {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        category: r.category,
        max_amount: r.max_amount,
        requires_receipt_above: r.requires_receipt_above,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct ExpensePolicyRepo;

impl ExpensePolicyRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ExpensePolicy>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<PolicyRow> = sqlx::query_as(
            "SELECT id, organization_id, category, max_amount, requires_receipt_above, created_at \
             FROM expense_policies WHERE organization_id = $1 ORDER BY category",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn upsert(
        pool: &PgPool,
        org_id: &str,
        category: &str,
        input: UpsertExpensePolicy,
    ) -> Result<ExpensePolicy, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO expense_policies (organization_id, category, max_amount, requires_receipt_above) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (organization_id, category) DO UPDATE \
               SET max_amount = EXCLUDED.max_amount, \
                   requires_receipt_above = EXCLUDED.requires_receipt_above \
             RETURNING id",
        )
        .bind(org_uuid)
        .bind(category)
        .bind(input.max_amount)
        .bind(input.requires_receipt_above)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: PolicyRow = sqlx::query_as(
            "SELECT id, organization_id, category, max_amount, requires_receipt_above, created_at \
             FROM expense_policies WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, category: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let n = sqlx::query(
            "DELETE FROM expense_policies WHERE organization_id = $1 AND category = $2",
        )
        .bind(org_uuid)
        .bind(category)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Returns Err(DbError::Conflict) if the expense violates a policy.
    pub async fn check(
        pool: &PgPool,
        org_id: &str,
        category: &str,
        amount: i64,
    ) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let policy: Option<PolicyRow> = sqlx::query_as(
            "SELECT id, organization_id, category, max_amount, requires_receipt_above, created_at \
             FROM expense_policies \
             WHERE organization_id = $1 AND category = $2",
        )
        .bind(org_uuid)
        .bind(category)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        if let Some(p) = policy {
            if amount > p.max_amount {
                return Err(DbError::Conflict(format!(
                    "expense amount exceeds the {} policy limit of {}",
                    category, p.max_amount
                )));
            }
        }
        Ok(())
    }
}
