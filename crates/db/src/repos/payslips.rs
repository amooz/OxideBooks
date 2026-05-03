use oxidebooks_core::models::{CreatePayslip, Payslip};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PayslipRow {
    id: Uuid,
    organization_id: Uuid,
    payroll_run_id: Uuid,
    employee_id: Uuid,
    gross_pay: i64,
    tax_withheld: i64,
    deductions: i64,
    net_pay: i64,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

impl From<PayslipRow> for Payslip {
    fn from(r: PayslipRow) -> Self {
        Payslip {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            payroll_run_id: r.payroll_run_id.to_string(),
            employee_id: r.employee_id.to_string(),
            gross_pay: r.gross_pay,
            tax_withheld: r.tax_withheld,
            deductions: r.deductions,
            net_pay: r.net_pay,
            notes: r.notes,
            created_at: r.created_at,
        }
    }
}

pub struct PayslipRepo;

impl PayslipRepo {
    pub async fn list_by_run(
        pool: &PgPool,
        org_id: &str,
        run_id: &str,
    ) -> Result<Vec<Payslip>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let run_uuid = parse_uuid(run_id)?;
        // Verify run belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM payroll_runs WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(run_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<PayslipRow> = sqlx::query_as(
            "SELECT id, organization_id, payroll_run_id, employee_id, gross_pay, \
             tax_withheld, deductions, net_pay, notes, created_at \
             FROM payslips WHERE payroll_run_id = $1 ORDER BY created_at ASC",
        )
        .bind(run_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(Payslip::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Payslip, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: PayslipRow = sqlx::query_as(
            "SELECT id, organization_id, payroll_run_id, employee_id, gross_pay, \
             tax_withheld, deductions, net_pay, notes, created_at \
             FROM payslips WHERE organization_id = $1 AND id = $2",
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
        run_id: &str,
        input: CreatePayslip,
    ) -> Result<Payslip, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let run_uuid = parse_uuid(run_id)?;
        let emp_uuid = parse_uuid(&input.employee_id)?;

        // Verify run belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM payroll_runs WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(run_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        // Verify employee belongs to org.
        let emp_exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM employees WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(emp_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if emp_exists.is_none() {
            return Err(DbError::Conflict(
                "employee not found in this organization".into(),
            ));
        }

        if input.gross_pay < 0 {
            return Err(DbError::Conflict("gross_pay must be non-negative".into()));
        }

        let net_pay = input.gross_pay - input.tax_withheld - input.deductions;
        if net_pay < 0 {
            return Err(DbError::Conflict(
                "net_pay would be negative (tax_withheld + deductions exceeds gross_pay)".into(),
            ));
        }

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO payslips \
             (id, organization_id, payroll_run_id, employee_id, gross_pay, \
              tax_withheld, deductions, net_pay, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(run_uuid)
        .bind(emp_uuid)
        .bind(input.gross_pay)
        .bind(input.tax_withheld)
        .bind(input.deductions)
        .bind(net_pay)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
