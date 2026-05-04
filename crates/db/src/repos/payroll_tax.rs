use oxidebooks_core::models::{CreatePayrollTaxLiability, PayPayrollTax, PayrollTaxLiability};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TaxRow {
    id: Uuid,
    organization_id: Uuid,
    payroll_run_id: Uuid,
    tax_type: String,
    employee_amount: i64,
    employer_amount: i64,
    period_start: Date,
    period_end: Date,
    due_date: Option<Date>,
    paid_date: Option<Date>,
    status: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TaxRow) -> PayrollTaxLiability {
    PayrollTaxLiability {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        payroll_run_id: r.payroll_run_id.to_string(),
        tax_type: r.tax_type,
        employee_amount: r.employee_amount,
        employer_amount: r.employer_amount,
        period_start: r.period_start,
        period_end: r.period_end,
        due_date: r.due_date,
        paid_date: r.paid_date,
        status: r.status,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, payroll_run_id, tax_type, employee_amount, \
                    employer_amount, period_start, period_end, due_date, paid_date, \
                    status, notes, created_at, updated_at";

pub struct PayrollTaxRepo;

impl PayrollTaxRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PayrollTaxLiability>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TaxRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payroll_tax_liabilities \
             WHERE organization_id = $1 ORDER BY period_end DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn list_for_run(
        pool: &PgPool,
        org_id: &str,
        run_id: &str,
    ) -> Result<Vec<PayrollTaxLiability>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let run_uuid = parse_uuid(run_id)?;
        let rows: Vec<TaxRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payroll_tax_liabilities \
             WHERE organization_id = $1 AND payroll_run_id = $2 ORDER BY tax_type ASC"
        ))
        .bind(org_uuid)
        .bind(run_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PayrollTaxLiability, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaxRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM payroll_tax_liabilities \
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
        input: CreatePayrollTaxLiability,
    ) -> Result<PayrollTaxLiability, DbError> {
        if input.employee_amount < 0 || input.employer_amount < 0 {
            return Err(DbError::Conflict("amounts must be non-negative".into()));
        }
        let org_uuid = parse_uuid(org_id)?;
        let run_uuid = parse_uuid(&input.payroll_run_id)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO payroll_tax_liabilities \
             (organization_id, payroll_run_id, tax_type, employee_amount, employer_amount, \
              period_start, period_end, due_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(run_uuid)
        .bind(&input.tax_type)
        .bind(input.employee_amount)
        .bind(input.employer_amount)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.due_date)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Marks a tax liability as paid.
    pub async fn mark_paid(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: PayPayrollTax,
    ) -> Result<PayrollTaxLiability, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE payroll_tax_liabilities \
             SET status = 'paid', paid_date = $1, notes = COALESCE($2, notes), \
                 updated_at = NOW() \
             WHERE organization_id = $3 AND id = $4 AND status = 'accrued'",
        )
        .bind(input.paid_date)
        .bind(&input.notes)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "tax liability must be in accrued status to mark paid".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PayrollTaxLiability, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE payroll_tax_liabilities SET status = 'voided', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status != 'voided'",
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
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
