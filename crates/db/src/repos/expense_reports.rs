use oxidebooks_core::models::{CreateExpenseReport, ExpenseReport, UpdateExpenseReport};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, title, employee_id, notes, status, total_amount, \
     approved_by, approved_at, reimbursed_at, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct ReportRow {
    id: Uuid,
    organization_id: Uuid,
    title: String,
    employee_id: Option<Uuid>,
    notes: Option<String>,
    status: String,
    total_amount: i64,
    approved_by: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
    reimbursed_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ReportRow> for ExpenseReport {
    fn from(r: ReportRow) -> Self {
        ExpenseReport {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            title: r.title,
            employee_id: r.employee_id.map(|u| u.to_string()),
            notes: r.notes,
            status: r.status,
            total_amount: r.total_amount,
            approved_by: r.approved_by.map(|u| u.to_string()),
            approved_at: r.approved_at,
            reimbursed_at: r.reimbursed_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct ExpenseReportRepo;

impl ExpenseReportRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        employee_id: Option<&str>,
    ) -> Result<Vec<ExpenseReport>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ReportRow> = if let Some(eid) = employee_id {
            let emp_uuid = parse_uuid(eid)?;
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM expense_reports \
                 WHERE organization_id = $1 AND employee_id = $2 \
                 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .bind(emp_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM expense_reports \
                 WHERE organization_id = $1 \
                 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        Ok(rows.into_iter().map(ExpenseReport::from).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: ReportRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM expense_reports \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(ExpenseReport::from(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateExpenseReport,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let emp_uuid = input.employee_id.as_deref().map(parse_uuid).transpose()?;
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO expense_reports \
             (id, organization_id, title, employee_id, notes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.title)
        .bind(emp_uuid)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateExpenseReport,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE expense_reports SET \
             title      = COALESCE($3, title), \
             notes      = COALESCE($4, notes), \
             updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(&input.title)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Attach an expense to this report (must be in 'draft' status and belong to org).
    pub async fn add_expense(
        pool: &PgPool,
        org_id: &str,
        report_id: &str,
        expense_id: &str,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let report_uuid = parse_uuid(report_id)?;
        let expense_uuid = parse_uuid(expense_id)?;

        let report = Self::get_by_id(pool, org_id, report_id).await?;
        if report.status != "draft" {
            return Err(DbError::Conflict(
                "can only add expenses to a draft report".into(),
            ));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE expenses SET expense_report_id = $1 \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(report_uuid)
        .bind(expense_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Recompute total_amount.
        sqlx::query(
            "UPDATE expense_reports SET \
             total_amount = (SELECT COALESCE(SUM(amount), 0) FROM expenses \
                             WHERE expense_report_id = $1), \
             updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(report_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, report_id).await
    }

    pub async fn submit(pool: &PgPool, org_id: &str, id: &str) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE expense_reports SET status = 'submitted', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "report must be in draft status to submit".into(),
            ));
        }

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        approver_id: &str,
        id: &str,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let approver_uuid = parse_uuid(approver_id)?;

        let rows = sqlx::query(
            "UPDATE expense_reports \
             SET status = 'approved', approved_by = $3, approved_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'submitted'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .bind(approver_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "report must be submitted before approval".into(),
            ));
        }

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn reimburse(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE expense_reports \
             SET status = 'reimbursed', reimbursed_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'approved'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "report must be approved before reimbursement".into(),
            ));
        }

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn reject(pool: &PgPool, org_id: &str, id: &str) -> Result<ExpenseReport, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE expense_reports SET status = 'rejected', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'submitted'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "report must be submitted before rejection".into(),
            ));
        }

        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
