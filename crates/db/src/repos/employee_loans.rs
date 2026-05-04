use oxidebooks_core::models::{
    CreateEmployeeLoan, CreateLoanRepayment, EmployeeLoan, LoanRepayment, UpdateEmployeeLoan,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct LoanRow {
    id: Uuid,
    organization_id: Uuid,
    employee_id: Uuid,
    amount: i64,
    balance: i64,
    purpose: Option<String>,
    account_id: Option<Uuid>,
    loan_date: Date,
    status: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn loan_from_row(r: LoanRow) -> EmployeeLoan {
    EmployeeLoan {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        employee_id: r.employee_id.to_string(),
        amount: r.amount,
        balance: r.balance,
        purpose: r.purpose,
        account_id: r.account_id.map(|u| u.to_string()),
        loan_date: r.loan_date,
        status: r.status,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct RepaymentRow {
    id: Uuid,
    loan_id: Uuid,
    repayment_date: Date,
    amount: i64,
    payslip_id: Option<Uuid>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

fn repayment_from_row(r: RepaymentRow) -> LoanRepayment {
    LoanRepayment {
        id: r.id.to_string(),
        loan_id: r.loan_id.to_string(),
        repayment_date: r.repayment_date,
        amount: r.amount,
        payslip_id: r.payslip_id.map(|u| u.to_string()),
        notes: r.notes,
        created_at: r.created_at,
    }
}

const LOAN_COLS: &str = "id, organization_id, employee_id, amount, balance, purpose, \
                         account_id, loan_date, status, notes, created_at, updated_at";

pub struct EmployeeLoanRepo;

impl EmployeeLoanRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<EmployeeLoan>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<LoanRow> = sqlx::query_as(&format!(
            "SELECT {LOAN_COLS} FROM employee_loans \
             WHERE organization_id = $1 ORDER BY loan_date DESC, id ASC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(loan_from_row).collect())
    }

    pub async fn list_for_employee(
        pool: &PgPool,
        org_id: &str,
        employee_id: &str,
    ) -> Result<Vec<EmployeeLoan>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(employee_id)?;
        let rows: Vec<LoanRow> = sqlx::query_as(&format!(
            "SELECT {LOAN_COLS} FROM employee_loans \
             WHERE organization_id = $1 AND employee_id = $2 \
             ORDER BY loan_date DESC, id ASC"
        ))
        .bind(org_uuid)
        .bind(emp_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(loan_from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<EmployeeLoan, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: LoanRow = sqlx::query_as(&format!(
            "SELECT {LOAN_COLS} FROM employee_loans WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(loan_from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateEmployeeLoan,
    ) -> Result<EmployeeLoan, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = parse_uuid(&input.employee_id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO employee_loans \
             (organization_id, employee_id, amount, balance, purpose, account_id, loan_date, notes) \
             VALUES ($1,$2,$3,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(emp_uuid)
        .bind(input.amount)
        .bind(&input.purpose)
        .bind(acct_uuid)
        .bind(input.loan_date)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateEmployeeLoan,
    ) -> Result<EmployeeLoan, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let acct_uuid = input.account_id.as_deref().map(parse_uuid).transpose()?;

        sqlx::query(
            "UPDATE employee_loans SET \
             purpose    = COALESCE($1, purpose), \
             account_id = COALESCE($2, account_id), \
             notes      = COALESCE($3, notes), \
             loan_date  = COALESCE($4, loan_date), \
             updated_at = NOW() \
             WHERE organization_id = $5 AND id = $6",
        )
        .bind(input.purpose)
        .bind(acct_uuid)
        .bind(input.notes)
        .bind(input.loan_date)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Records a repayment, decrements loan balance, marks paid_off when balance reaches 0.
    pub async fn record_repayment(
        pool: &PgPool,
        org_id: &str,
        loan_id: &str,
        input: CreateLoanRepayment,
    ) -> Result<LoanRepayment, DbError> {
        if input.amount <= 0 {
            return Err(DbError::Conflict("amount must be positive".into()));
        }
        let loan = Self::get_by_id(pool, org_id, loan_id).await?;
        if loan.status == "paid_off" || loan.status == "written_off" {
            return Err(DbError::Conflict(format!(
                "cannot repay a {} loan",
                loan.status
            )));
        }
        if input.amount > loan.balance {
            return Err(DbError::Conflict(format!(
                "repayment {} exceeds balance {}",
                input.amount, loan.balance
            )));
        }

        let loan_uuid = parse_uuid(loan_id)?;
        let payslip_uuid = input.payslip_id.as_deref().map(parse_uuid).transpose()?;

        let rep_id: Uuid = sqlx::query_scalar(
            "INSERT INTO employee_loan_repayments \
             (loan_id, repayment_date, amount, payslip_id, notes) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(loan_uuid)
        .bind(input.repayment_date)
        .bind(input.amount)
        .bind(payslip_uuid)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let new_balance = loan.balance - input.amount;
        let new_status = if new_balance == 0 {
            "paid_off"
        } else {
            "active"
        };
        let org_uuid = parse_uuid(org_id)?;
        sqlx::query(
            "UPDATE employee_loans SET balance = $1, status = $2, updated_at = NOW() \
             WHERE organization_id = $3 AND id = $4",
        )
        .bind(new_balance)
        .bind(new_status)
        .bind(org_uuid)
        .bind(loan_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: RepaymentRow = sqlx::query_as(
            "SELECT id, loan_id, repayment_date, amount, payslip_id, notes, created_at \
             FROM employee_loan_repayments WHERE id = $1",
        )
        .bind(rep_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(repayment_from_row(row))
    }

    pub async fn list_repayments(
        pool: &PgPool,
        org_id: &str,
        loan_id: &str,
    ) -> Result<Vec<LoanRepayment>, DbError> {
        // Verify loan belongs to org.
        Self::get_by_id(pool, org_id, loan_id).await?;
        let loan_uuid = parse_uuid(loan_id)?;
        let rows: Vec<RepaymentRow> = sqlx::query_as(
            "SELECT id, loan_id, repayment_date, amount, payslip_id, notes, created_at \
             FROM employee_loan_repayments WHERE loan_id = $1 ORDER BY repayment_date ASC",
        )
        .bind(loan_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(repayment_from_row).collect())
    }

    pub async fn write_off(pool: &PgPool, org_id: &str, id: &str) -> Result<EmployeeLoan, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE employee_loans SET status = 'written_off', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'active'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "only active loans can be written off".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
