use oxidebooks_core::models::{
    CreateEmployeeBankAccount, EmployeeBankAccount, UpdateEmployeeBankAccount,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    organization_id: Uuid,
    employee_id: Uuid,
    bank_name: String,
    routing_number: String,
    account_last4: String,
    account_type: String,
    is_primary: bool,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

const COLS: &str = "id, organization_id, employee_id, bank_name, routing_number, \
    account_last4, account_type, is_primary, is_active, created_at, updated_at";

fn from_row(r: Row) -> EmployeeBankAccount {
    EmployeeBankAccount {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        employee_id: r.employee_id.to_string(),
        bank_name: r.bank_name,
        routing_number: r.routing_number,
        account_last4: r.account_last4,
        account_type: r.account_type,
        is_primary: r.is_primary,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct EmployeeBankAccountRepo;

impl EmployeeBankAccountRepo {
    pub async fn list_for_employee(
        pool: &PgPool,
        org_id: &str,
        employee_id: &str,
    ) -> Result<Vec<EmployeeBankAccount>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(employee_id)?;
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM employee_bank_accounts \
             WHERE organization_id = $1 AND employee_id = $2 \
             ORDER BY is_primary DESC, created_at ASC"
        ))
        .bind(org_uuid)
        .bind(emp_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<EmployeeBankAccount, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: Row = sqlx::query_as(&format!(
            "SELECT {COLS} FROM employee_bank_accounts \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        employee_id: &str,
        input: CreateEmployeeBankAccount,
    ) -> Result<EmployeeBankAccount, DbError> {
        if input.account_number.len() < 4 {
            return Err(DbError::Conflict(
                "account_number must be at least 4 digits".into(),
            ));
        }
        if !input.account_number.chars().all(|c| c.is_ascii_digit()) {
            return Err(DbError::Conflict(
                "account_number must contain only digits".into(),
            ));
        }
        validate_account_type(&input.account_type)?;

        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(employee_id)?;
        let last4 = &input.account_number[input.account_number.len() - 4..];

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // If this is primary, demote any existing primary first.
        if input.is_primary {
            sqlx::query(
                "UPDATE employee_bank_accounts SET is_primary = FALSE \
                 WHERE organization_id = $1 AND employee_id = $2 AND is_primary = TRUE",
            )
            .bind(org_uuid)
            .bind(emp_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO employee_bank_accounts \
             (organization_id, employee_id, bank_name, routing_number, account_last4, \
              account_type, is_primary) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(emp_uuid)
        .bind(&input.bank_name)
        .bind(&input.routing_number)
        .bind(last4)
        .bind(&input.account_type)
        .bind(input.is_primary)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateEmployeeBankAccount,
    ) -> Result<EmployeeBankAccount, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let account = Self::get_by_id(pool, org_id, id).await?;
        let emp_uuid = parse_uuid(&account.employee_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        if input.is_primary == Some(true) {
            sqlx::query(
                "UPDATE employee_bank_accounts SET is_primary = FALSE \
                 WHERE organization_id = $1 AND employee_id = $2 AND is_primary = TRUE AND id <> $3",
            )
            .bind(org_uuid)
            .bind(emp_uuid)
            .bind(id_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query(
            "UPDATE employee_bank_accounts SET \
             bank_name  = COALESCE($3, bank_name), \
             is_primary = COALESCE($4, is_primary), \
             is_active  = COALESCE($5, is_active), \
             updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.bank_name)
        .bind(input.is_primary)
        .bind(input.is_active)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "DELETE FROM employee_bank_accounts WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn validate_account_type(at: &str) -> Result<(), DbError> {
    match at {
        "checking" | "savings" => Ok(()),
        other => Err(DbError::Conflict(format!("invalid account_type '{other}'"))),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
