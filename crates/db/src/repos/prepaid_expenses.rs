use oxidebooks_core::models::{
    CreateJournalEntry, CreateJournalLine, CreatePrepaidExpenseSchedule, PrepaidExpenseEntry,
    PrepaidExpenseSchedule, UpdatePrepaidExpenseSchedule,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::TransactionRepo;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    id: Uuid,
    organization_id: Uuid,
    description: String,
    total_amount: i64,
    asset_account_id: Uuid,
    expense_account_id: Uuid,
    start_date: Date,
    end_date: Date,
    frequency: String,
    is_active: bool,
    amortized_amount: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    schedule_id: Uuid,
    period_date: Date,
    amount: i64,
    journal_entry_id: Option<Uuid>,
    recognized_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

async fn load_entries(pool: &PgPool, sid: Uuid) -> Result<Vec<PrepaidExpenseEntry>, DbError> {
    let rows: Vec<EntryRow> = sqlx::query_as(
        "SELECT id, schedule_id, period_date, amount, journal_entry_id, recognized_at, created_at
         FROM prepaid_expense_entries WHERE schedule_id = $1 ORDER BY period_date",
    )
    .bind(sid)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| PrepaidExpenseEntry {
            id: r.id.to_string(),
            schedule_id: r.schedule_id.to_string(),
            period_date: r.period_date,
            amount: r.amount,
            journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
            recognized_at: r.recognized_at,
            created_at: r.created_at,
        })
        .collect())
}

fn schedule_from_row(r: ScheduleRow, entries: Vec<PrepaidExpenseEntry>) -> PrepaidExpenseSchedule {
    PrepaidExpenseSchedule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        description: r.description,
        total_amount: r.total_amount,
        asset_account_id: r.asset_account_id.to_string(),
        expense_account_id: r.expense_account_id.to_string(),
        start_date: r.start_date,
        end_date: r.end_date,
        frequency: r.frequency,
        is_active: r.is_active,
        amortized_amount: r.amortized_amount,
        entries,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

/// Build a list of monthly period start dates between start_date and end_date.
fn monthly_periods(start: Date, end: Date) -> Vec<Date> {
    let mut dates = Vec::new();
    let mut cur = start;
    while cur <= end {
        dates.push(cur);
        // Advance by one month
        let next_month = cur.month().next();
        let next_year = if next_month == time::Month::January {
            cur.year() + 1
        } else {
            cur.year()
        };
        let days_in_next = next_month.length(next_year);
        let day = cur.day().min(days_in_next);
        cur = Date::from_calendar_date(next_year, next_month, day)
            .unwrap_or(end + time::Duration::days(1));
    }
    dates
}

pub struct PrepaidExpenseRepo;

impl PrepaidExpenseRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PrepaidExpenseSchedule>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<ScheduleRow> = sqlx::query_as(
            "SELECT id, organization_id, description, total_amount, asset_account_id,
                    expense_account_id, start_date, end_date, frequency, is_active,
                    amortized_amount, created_at, updated_at
             FROM prepaid_expense_schedules
             WHERE organization_id = $1 ORDER BY start_date DESC",
        )
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let entries = load_entries(pool, row.id).await?;
            result.push(schedule_from_row(row, entries));
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PrepaidExpenseSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let row: ScheduleRow = sqlx::query_as(
            "SELECT id, organization_id, description, total_amount, asset_account_id,
                    expense_account_id, start_date, end_date, frequency, is_active,
                    amortized_amount, created_at, updated_at
             FROM prepaid_expense_schedules WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let entries = load_entries(pool, row.id).await?;
        Ok(schedule_from_row(row, entries))
    }

    /// Create a prepaid expense schedule and pre-generate monthly amortization entries.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePrepaidExpenseSchedule,
    ) -> Result<PrepaidExpenseSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let asset_acc = parse_uuid(&input.asset_account_id)?;
        let expense_acc = parse_uuid(&input.expense_account_id)?;

        if input.end_date <= input.start_date {
            return Err(DbError::Conflict(
                "end_date must be after start_date".into(),
            ));
        }
        if input.total_amount <= 0 {
            return Err(DbError::Conflict("total_amount must be positive".into()));
        }

        let periods = monthly_periods(input.start_date, input.end_date);
        if periods.is_empty() {
            return Err(DbError::Conflict("no periods found in date range".into()));
        }

        let sid: Uuid = sqlx::query_scalar(
            "INSERT INTO prepaid_expense_schedules
                (organization_id, description, total_amount, asset_account_id, expense_account_id,
                 start_date, end_date, frequency)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
        )
        .bind(org)
        .bind(&input.description)
        .bind(input.total_amount)
        .bind(asset_acc)
        .bind(expense_acc)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(&input.frequency)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Distribute total_amount evenly across periods; last period gets remainder
        let n = periods.len() as i64;
        let per_period = input.total_amount / n;
        let remainder = input.total_amount - per_period * n;

        for (i, &period_date) in periods.iter().enumerate() {
            let amount = if i == periods.len() - 1 {
                per_period + remainder
            } else {
                per_period
            };
            sqlx::query(
                "INSERT INTO prepaid_expense_entries (schedule_id, period_date, amount)
                 VALUES ($1, $2, $3)",
            )
            .bind(sid)
            .bind(period_date)
            .bind(amount)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, org_id, &sid.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdatePrepaidExpenseSchedule,
    ) -> Result<PrepaidExpenseSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE prepaid_expense_schedules SET
             is_active   = COALESCE($3, is_active),
             description = COALESCE($4, description),
             updated_at  = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(sid)
        .bind(input.is_active)
        .bind(&input.description)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Recognize a single amortization entry: post a JE and mark it recognized.
    pub async fn recognize(
        pool: &PgPool,
        org_id: &str,
        entry_id: &str,
    ) -> Result<PrepaidExpenseEntry, DbError> {
        let org = parse_uuid(org_id)?;
        let eid = parse_uuid(entry_id)?;

        // Load entry + schedule
        let entry: EntryRow = sqlx::query_as(
            "SELECT e.id, e.schedule_id, e.period_date, e.amount, e.journal_entry_id,
                    e.recognized_at, e.created_at
             FROM prepaid_expense_entries e
             JOIN prepaid_expense_schedules s ON s.id = e.schedule_id
             WHERE s.organization_id = $1 AND e.id = $2",
        )
        .bind(org)
        .bind(eid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if entry.recognized_at.is_some() {
            return Err(DbError::Conflict("entry is already recognized".into()));
        }

        let schedule: ScheduleRow = sqlx::query_as(
            "SELECT id, organization_id, description, total_amount, asset_account_id,
                    expense_account_id, start_date, end_date, frequency, is_active,
                    amortized_amount, created_at, updated_at
             FROM prepaid_expense_schedules WHERE id = $1",
        )
        .bind(entry.schedule_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Post a JE: Dr expense_account / Cr asset_account
        let je = CreateJournalEntry {
            date: entry.period_date,
            description: format!(
                "Prepaid amortization: {} ({})",
                schedule.description, entry.period_date
            ),
            reference: None,
            lines: vec![
                CreateJournalLine {
                    account_id: schedule.expense_account_id.to_string(),
                    debit: entry.amount,
                    credit: 0,
                    description: None,
                },
                CreateJournalLine {
                    account_id: schedule.asset_account_id.to_string(),
                    debit: 0,
                    credit: entry.amount,
                    description: None,
                },
            ],
        };

        let posted = TransactionRepo::create_posted(pool, org_id, "system", je).await?;
        let je_uuid = parse_uuid(&posted.id)?;

        let now = OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE prepaid_expense_entries
             SET journal_entry_id = $1, recognized_at = $2
             WHERE id = $3",
        )
        .bind(je_uuid)
        .bind(now)
        .bind(eid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Update schedule amortized_amount
        sqlx::query(
            "UPDATE prepaid_expense_schedules
             SET amortized_amount = amortized_amount + $1, updated_at = now()
             WHERE id = $2",
        )
        .bind(entry.amount)
        .bind(entry.schedule_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(PrepaidExpenseEntry {
            id: entry.id.to_string(),
            schedule_id: entry.schedule_id.to_string(),
            period_date: entry.period_date,
            amount: entry.amount,
            journal_entry_id: Some(posted.id),
            recognized_at: Some(now),
            created_at: entry.created_at,
        })
    }
}
