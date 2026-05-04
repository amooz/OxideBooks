use oxidebooks_core::models::{
    CreateJournalEntry, CreateJournalLine, CreatePayrollEntry, CreatePayrollRun, PayrollEntry,
    PayrollRun, PayrollRunSummary,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    error::{map_sqlx_err, DbError},
    repos::TransactionRepo,
};

#[derive(sqlx::FromRow)]
struct PayrollRunRow {
    id: Uuid,
    organization_id: Uuid,
    period_start: Date,
    period_end: Date,
    status: String,
    journal_entry_id: Option<Uuid>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct PayrollEntryRow {
    id: Uuid,
    payroll_run_id: Uuid,
    user_id: Uuid,
    gross_pay: i64,
    tax_withheld: i64,
    other_deductions: i64,
    net_pay: i64,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

fn run_from_row(r: PayrollRunRow) -> PayrollRun {
    PayrollRun {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        period_start: r.period_start,
        period_end: r.period_end,
        status: r.status,
        journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn entry_from_row(r: PayrollEntryRow) -> PayrollEntry {
    PayrollEntry {
        id: r.id.to_string(),
        payroll_run_id: r.payroll_run_id.to_string(),
        user_id: r.user_id.to_string(),
        gross_pay: r.gross_pay,
        tax_withheld: r.tax_withheld,
        other_deductions: r.other_deductions,
        net_pay: r.net_pay,
        notes: r.notes,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct PayrollRepo;

impl PayrollRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PayrollRun>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<PayrollRunRow> = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE organization_id = $1 ORDER BY period_start DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(run_from_row).collect())
    }

    pub async fn get(pool: &PgPool, org_id: &str, id: &str) -> Result<PayrollRunSummary, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let run_row: PayrollRunRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let entry_rows: Vec<PayrollEntryRow> = sqlx::query_as(
            "SELECT id, payroll_run_id, user_id, gross_pay, tax_withheld, \
             other_deductions, net_pay, notes, created_at \
             FROM payroll_entries WHERE payroll_run_id = $1 ORDER BY created_at",
        )
        .bind(id_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let entries: Vec<PayrollEntry> = entry_rows.into_iter().map(entry_from_row).collect();
        let total_gross: i64 = entries.iter().map(|e| e.gross_pay).sum();
        let total_net: i64 = entries.iter().map(|e| e.net_pay).sum();
        let total_tax: i64 = entries.iter().map(|e| e.tax_withheld).sum();

        Ok(PayrollRunSummary {
            run: run_from_row(run_row),
            entries,
            total_gross,
            total_net,
            total_tax,
        })
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePayrollRun,
    ) -> Result<PayrollRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO payroll_runs (organization_id, period_start, period_end, notes) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(org_uuid)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: PayrollRunRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(run_from_row(row))
    }

    pub async fn add_entry(
        pool: &PgPool,
        org_id: &str,
        run_id: &str,
        input: CreatePayrollEntry,
    ) -> Result<PayrollEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let run_uuid = parse_uuid(run_id)?;
        let user_uuid = parse_uuid(&input.user_id)?;

        // Ensure run belongs to org and is still draft
        let status: String = sqlx::query_scalar(
            "SELECT status FROM payroll_runs WHERE id = $1 AND organization_id = $2",
        )
        .bind(run_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if status != "draft" {
            return Err(DbError::Conflict(
                "can only add entries to a draft payroll run".to_string(),
            ));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO payroll_entries \
             (payroll_run_id, user_id, gross_pay, tax_withheld, other_deductions, notes) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(run_uuid)
        .bind(user_uuid)
        .bind(input.gross_pay)
        .bind(input.tax_withheld)
        .bind(input.other_deductions)
        .bind(input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: PayrollEntryRow = sqlx::query_as(
            "SELECT id, payroll_run_id, user_id, gross_pay, tax_withheld, \
             other_deductions, net_pay, notes, created_at \
             FROM payroll_entries WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(entry_from_row(row))
    }

    pub async fn approve(pool: &PgPool, org_id: &str, id: &str) -> Result<PayrollRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE payroll_runs SET status = 'approved', updated_at = now() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "payroll run not found or not in draft status".to_string(),
            ));
        }
        let row: PayrollRunRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(run_from_row(row))
    }

    /// POST /payroll-runs/:id/post-journal
    ///
    /// Creates a posted journal entry for the payroll run:
    ///   Debit  wages_account   gross pay
    ///   Credit tax_account     total tax withheld
    ///   Credit deductions_account other deductions (if > 0)
    ///   Credit cash_account    total net pay
    ///
    /// Idempotent: returns the existing journal entry if already posted.
    pub async fn post_journal(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        wages_account_id: &str,
        tax_account_id: &str,
        cash_account_id: &str,
        deductions_account_id: Option<&str>,
    ) -> Result<PayrollRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let summary = Self::get(pool, org_id, id).await?;
        if summary.run.journal_entry_id.is_some() {
            return Ok(summary.run);
        }
        if summary.run.status != "paid" && summary.run.status != "approved" {
            return Err(DbError::Conflict(
                "payroll run must be approved or paid before posting journal".into(),
            ));
        }
        if summary.total_gross == 0 {
            return Err(DbError::Conflict("no payroll entries to post".into()));
        }

        let period_end = summary.run.period_end;
        let description = format!(
            "Payroll {}-{}",
            summary.run.period_start, summary.run.period_end
        );

        let mut lines: Vec<CreateJournalLine> = vec![
            CreateJournalLine {
                account_id: wages_account_id.to_string(),
                description: Some("Gross wages".to_string()),
                debit: summary.total_gross,
                credit: 0,
            },
            CreateJournalLine {
                account_id: tax_account_id.to_string(),
                description: Some("Payroll tax withheld".to_string()),
                debit: 0,
                credit: summary.total_tax,
            },
            CreateJournalLine {
                account_id: cash_account_id.to_string(),
                description: Some("Net pay disbursed".to_string()),
                debit: 0,
                credit: summary.total_net,
            },
        ];

        let deductions = summary.total_gross - summary.total_tax - summary.total_net;
        if deductions > 0 {
            let ded_account = deductions_account_id.unwrap_or(tax_account_id);
            lines.push(CreateJournalLine {
                account_id: ded_account.to_string(),
                description: Some("Other payroll deductions".to_string()),
                debit: 0,
                credit: deductions,
            });
        }

        let je_input = CreateJournalEntry {
            date: period_end,
            reference: Some(format!("PAYROLL-{}", &id[..8])),
            description,
            lines,
            auto_reversal_date: None,
        };

        let je = TransactionRepo::create_posted(pool, org_id, "system", je_input).await?;

        let je_uuid = parse_uuid(&je.id)?;
        sqlx::query(
            "UPDATE payroll_runs SET journal_entry_id = $3, updated_at = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(je_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: PayrollRunRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(run_from_row(row))
    }

    pub async fn mark_paid(pool: &PgPool, org_id: &str, id: &str) -> Result<PayrollRun, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE payroll_runs SET status = 'paid', updated_at = now() \
             WHERE id = $1 AND organization_id = $2 AND status = 'approved'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "payroll run not found or not in approved status".to_string(),
            ));
        }
        let row: PayrollRunRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, status, \
             journal_entry_id, notes, created_at, updated_at \
             FROM payroll_runs WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(run_from_row(row))
    }
}
