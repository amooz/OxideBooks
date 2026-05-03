use oxidebooks_core::models::{CreateEmployee, Employee, UpdateEmployee};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct EmployeeRow {
    id: Uuid,
    organization_id: Uuid,
    first_name: String,
    last_name: String,
    email: Option<String>,
    employee_number: Option<String>,
    start_date: Date,
    terminated_at: Option<Date>,
    pay_type: String,
    pay_rate: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<EmployeeRow> for Employee {
    fn from(r: EmployeeRow) -> Self {
        Employee {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            employee_number: r.employee_number,
            start_date: r.start_date,
            terminated_at: r.terminated_at,
            pay_type: r.pay_type,
            pay_rate: r.pay_rate,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct EmployeeRepo;

impl EmployeeRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Employee>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<EmployeeRow> = sqlx::query_as(
            "SELECT id, organization_id, first_name, last_name, email, employee_number, \
             start_date, terminated_at, pay_type, pay_rate, created_at, updated_at \
             FROM employees WHERE organization_id = $1 \
             ORDER BY last_name ASC, first_name ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(Employee::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Employee, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: EmployeeRow = sqlx::query_as(
            "SELECT id, organization_id, first_name, last_name, email, employee_number, \
             start_date, terminated_at, pay_type, pay_rate, created_at, updated_at \
             FROM employees WHERE organization_id = $1 AND id = $2",
        )
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
        input: CreateEmployee,
    ) -> Result<Employee, DbError> {
        if input.pay_type != "salary" && input.pay_type != "hourly" {
            return Err(DbError::Conflict(
                "pay_type must be 'salary' or 'hourly'".into(),
            ));
        }
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO employees \
             (id, organization_id, first_name, last_name, email, employee_number, \
              start_date, pay_type, pay_rate) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.employee_number)
        .bind(input.start_date)
        .bind(&input.pay_type)
        .bind(input.pay_rate)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateEmployee,
    ) -> Result<Employee, DbError> {
        if let Some(ref pt) = input.pay_type {
            if pt != "salary" && pt != "hourly" {
                return Err(DbError::Conflict(
                    "pay_type must be 'salary' or 'hourly'".into(),
                ));
            }
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows_affected = sqlx::query(
            "UPDATE employees SET \
             first_name    = COALESCE($3, first_name), \
             last_name     = COALESCE($4, last_name), \
             email         = COALESCE($5, email), \
             employee_number = COALESCE($6, employee_number), \
             pay_type      = COALESCE($7, pay_type), \
             pay_rate      = COALESCE($8, pay_rate), \
             terminated_at = COALESCE($9, terminated_at), \
             updated_at    = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.employee_number)
        .bind(&input.pay_type)
        .bind(input.pay_rate)
        .bind(input.terminated_at)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows_affected =
            sqlx::query("DELETE FROM employees WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(id_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();
        if rows_affected == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
