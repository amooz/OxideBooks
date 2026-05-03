use oxidebooks_core::models::{CreateExpenseCategory, ExpenseCategory, UpdateExpenseCategory};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, name, description, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<CategoryRow> for ExpenseCategory {
    fn from(r: CategoryRow) -> Self {
        ExpenseCategory {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct ExpenseCategoryRepo;

impl ExpenseCategoryRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ExpenseCategory>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<CategoryRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expense_categories WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(ExpenseCategory::from).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ExpenseCategory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: CategoryRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expense_categories WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(row.into())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateExpenseCategory,
    ) -> Result<ExpenseCategory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO expense_categories (id, organization_id, name, description) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        let row: CategoryRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expense_categories WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateExpenseCategory,
    ) -> Result<ExpenseCategory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE expense_categories SET \
             name        = COALESCE($3, name), \
             description = COALESCE($4, description), \
             updated_at  = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(input.name)
        .bind(input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        let row: CategoryRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expense_categories WHERE id = $1"
        ))
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n =
            sqlx::query("DELETE FROM expense_categories WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(id_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}
