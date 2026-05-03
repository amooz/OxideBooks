use oxidebooks_core::models::{CreateDepartment, Department, DepartmentPlReport, UpdateDepartment};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct DeptRow {
    id: Uuid,
    organization_id: Uuid,
    code: String,
    name: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: DeptRow) -> Department {
    Department {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        code: r.code,
        name: r.name,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct DepartmentRepo;

impl DepartmentRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Department>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<DeptRow> = sqlx::query_as(
            "SELECT id, organization_id, code, name, is_active, created_at, updated_at \
             FROM departments WHERE organization_id = $1 ORDER BY code ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Department, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: DeptRow = sqlx::query_as(
            "SELECT id, organization_id, code, name, is_active, created_at, updated_at \
             FROM departments WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateDepartment,
    ) -> Result<Department, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO departments (organization_id, code, name) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.code)
        .bind(&input.name)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateDepartment,
    ) -> Result<Department, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        Self::get_by_id(pool, org_id, id).await?;
        sqlx::query(
            "UPDATE departments SET \
             code      = COALESCE($1, code), \
             name      = COALESCE($2, name), \
             is_active = COALESCE($3, is_active), \
             updated_at = NOW() \
             WHERE id = $4 AND organization_id = $5",
        )
        .bind(input.code)
        .bind(input.name)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM departments WHERE id = $1 AND organization_id = $2")
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

    /// P&L scoped to a single department:
    /// revenue = sum of invoice line amounts for the department
    /// expenses = sum of expenses tagged with the department
    pub async fn department_pl(
        pool: &PgPool,
        org_id: &str,
        dept_id: &str,
        from: Date,
        to: Date,
    ) -> Result<DepartmentPlReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let dept_uuid = parse_uuid(dept_id)?;

        let dept = Self::get_by_id(pool, org_id, dept_id).await?;

        let revenue: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(il.quantity * il.unit_price / 100), 0)::BIGINT \
             FROM invoice_lines il \
             JOIN invoices i ON i.id = il.invoice_id \
             WHERE i.organization_id = $1 \
               AND il.department_id = $2 \
               AND i.date >= $3 AND i.date <= $4 \
               AND i.status NOT IN ('draft', 'voided')",
        )
        .bind(org_uuid)
        .bind(dept_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let expenses: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT \
             FROM expenses \
             WHERE organization_id = $1 \
               AND department_id = $2 \
               AND date >= $3 AND date <= $4 \
               AND status != 'rejected'",
        )
        .bind(org_uuid)
        .bind(dept_uuid)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(DepartmentPlReport {
            department_id: dept_id.to_string(),
            department_name: dept.name,
            revenue,
            expenses,
            net: revenue - expenses,
        })
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
