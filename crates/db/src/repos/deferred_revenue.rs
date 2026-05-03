use oxidebooks_core::models::{
    CreateDeferredRevenueSchedule, DeferredRevenueEntry, DeferredRevenueSchedule,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Option<Uuid>,
    invoice_line_id: Option<Uuid>,
    deferred_account_id: Uuid,
    revenue_account_id: Uuid,
    description: String,
    total_amount: i64,
    recognized_amount: i64,
    start_date: Date,
    end_date: Date,
    frequency: String,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    schedule_id: Uuid,
    recognition_date: Date,
    amount: i64,
    journal_entry_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

fn entry_from_row(r: EntryRow) -> DeferredRevenueEntry {
    DeferredRevenueEntry {
        id: r.id.to_string(),
        schedule_id: r.schedule_id.to_string(),
        recognition_date: r.recognition_date,
        amount: r.amount,
        journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
        created_at: r.created_at,
    }
}

async fn fetch_entries(
    pool: &PgPool,
    schedule_id: Uuid,
) -> Result<Vec<DeferredRevenueEntry>, DbError> {
    let rows: Vec<EntryRow> = sqlx::query_as(
        "SELECT id, schedule_id, recognition_date, amount, journal_entry_id, created_at
         FROM deferred_revenue_entries WHERE schedule_id = $1
         ORDER BY recognition_date, id",
    )
    .bind(schedule_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(entry_from_row).collect())
}

async fn schedule_from_row(
    pool: &PgPool,
    r: ScheduleRow,
) -> Result<DeferredRevenueSchedule, DbError> {
    let entries = fetch_entries(pool, r.id).await?;
    let remaining = r.total_amount - r.recognized_amount;
    Ok(DeferredRevenueSchedule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        invoice_line_id: r.invoice_line_id.map(|u| u.to_string()),
        deferred_account_id: r.deferred_account_id.to_string(),
        revenue_account_id: r.revenue_account_id.to_string(),
        description: r.description,
        total_amount: r.total_amount,
        recognized_amount: r.recognized_amount,
        remaining_amount: remaining,
        start_date: r.start_date,
        end_date: r.end_date,
        frequency: r.frequency,
        status: r.status,
        entries,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const COLS: &str = "id, organization_id, invoice_id, invoice_line_id, deferred_account_id,
     revenue_account_id, description, total_amount, recognized_amount, start_date,
     end_date, frequency, status, created_at, updated_at";

/// Generate evenly-spaced recognition entries based on frequency.
fn generate_entries(total: i64, start: Date, end: Date, frequency: &str) -> Vec<(Date, i64)> {
    use time::Month;

    let mut dates: Vec<Date> = Vec::new();
    let mut current = start;
    while current <= end {
        dates.push(current);
        current = match frequency {
            "daily" => current.next_day().unwrap_or(end),
            "weekly" => {
                let d = current + time::Duration::weeks(1);
                if d > end {
                    break;
                } else {
                    d
                }
            }
            "quarterly" => {
                let (y, m, d) = (current.year(), current.month() as u8, current.day());
                let new_m = m + 3;
                let (ny, nm) = if new_m > 12 {
                    (y + 1, Month::try_from(new_m - 12).unwrap_or(Month::January))
                } else {
                    (y, Month::try_from(new_m).unwrap_or(Month::December))
                };
                match Date::from_calendar_date(ny, nm, d.min(28)) {
                    Ok(nd) => nd,
                    Err(_) => break,
                }
            }
            "annually" => {
                match Date::from_calendar_date(
                    current.year() + 1,
                    current.month(),
                    current.day().min(28),
                ) {
                    Ok(nd) => nd,
                    Err(_) => break,
                }
            }
            _ => {
                // monthly (default)
                let (y, m, d) = (current.year(), current.month() as u8, current.day());
                let (ny, nm) = if m == 12 {
                    (y + 1, Month::January)
                } else {
                    (y, Month::try_from(m + 1).unwrap_or(Month::December))
                };
                match Date::from_calendar_date(ny, nm, d.min(28)) {
                    Ok(nd) => nd,
                    Err(_) => break,
                }
            }
        };
    }
    if dates.is_empty() {
        return vec![(start, total)];
    }
    let n = dates.len() as i64;
    let per_period = total / n;
    let remainder = total - per_period * n;
    dates
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            let amt = if i == 0 {
                per_period + remainder
            } else {
                per_period
            };
            (d, amt)
        })
        .collect()
}

pub struct DeferredRevenueRepo;

impl DeferredRevenueRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<DeferredRevenueSchedule>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deferred_revenue_schedules
             WHERE organization_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
             ORDER BY start_date DESC, created_at DESC"
        ))
        .bind(org)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(schedule_from_row(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<DeferredRevenueSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let row: ScheduleRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deferred_revenue_schedules
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        schedule_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateDeferredRevenueSchedule,
    ) -> Result<DeferredRevenueSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let id = Uuid::new_v4();
        let deferred_acct = parse_uuid(&input.deferred_account_id)?;
        let revenue_acct = parse_uuid(&input.revenue_account_id)?;
        let invoice_id = input.invoice_id.as_deref().map(parse_uuid).transpose()?;
        let invoice_line_id = input
            .invoice_line_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let valid_freqs = ["daily", "weekly", "monthly", "quarterly", "annually"];
        if !valid_freqs.contains(&input.frequency.as_str()) {
            return Err(DbError::Conflict(format!(
                "invalid frequency '{}'",
                input.frequency
            )));
        }

        if input.end_date <= input.start_date {
            return Err(DbError::Conflict(
                "end_date must be after start_date".into(),
            ));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO deferred_revenue_schedules
                (id, organization_id, invoice_id, invoice_line_id, deferred_account_id,
                 revenue_account_id, description, total_amount, start_date, end_date, frequency)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(org)
        .bind(invoice_id)
        .bind(invoice_line_id)
        .bind(deferred_acct)
        .bind(revenue_acct)
        .bind(&input.description)
        .bind(input.total_amount)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(&input.frequency)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let periods = generate_entries(
            input.total_amount,
            input.start_date,
            input.end_date,
            &input.frequency,
        );
        for (date, amount) in periods {
            sqlx::query(
                "INSERT INTO deferred_revenue_entries (schedule_id, recognition_date, amount)
                 VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(date)
            .bind(amount)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Mark a single entry as recognized and update the schedule totals.
    pub async fn recognize(
        pool: &PgPool,
        org_id: &str,
        schedule_id: &str,
        entry_id: &str,
    ) -> Result<DeferredRevenueSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(schedule_id)?;
        let eid = parse_uuid(entry_id)?;

        // Fetch the entry amount
        let amount: Option<i64> = sqlx::query_scalar(
            "SELECT amount FROM deferred_revenue_entries
             WHERE id = $1 AND schedule_id = $2 AND journal_entry_id IS NULL",
        )
        .bind(eid)
        .bind(sid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let amount = amount
            .ok_or_else(|| DbError::Conflict("entry already recognized or not found".into()))?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Mark entry as recognized (use a sentinel journal_entry_id = schedule_id for now;
        // real implementation would create a journal entry)
        sqlx::query(
            "UPDATE deferred_revenue_entries
             SET journal_entry_id = $3
             WHERE id = $1 AND schedule_id = $2",
        )
        .bind(eid)
        .bind(sid)
        .bind(sid) // reuse schedule uuid as sentinel
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Update schedule recognized_amount and status
        let new_recognized: i64 = sqlx::query_scalar(
            "UPDATE deferred_revenue_schedules
             SET recognized_amount = recognized_amount + $2, updated_at = now()
             WHERE organization_id = $3 AND id = $1
             RETURNING recognized_amount",
        )
        .bind(sid)
        .bind(amount)
        .bind(org)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let total: i64 =
            sqlx::query_scalar("SELECT total_amount FROM deferred_revenue_schedules WHERE id = $1")
                .bind(sid)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;

        if new_recognized >= total {
            sqlx::query(
                "UPDATE deferred_revenue_schedules SET status = 'completed', updated_at = now()
                 WHERE id = $1",
            )
            .bind(sid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, schedule_id).await
    }

    pub async fn cancel(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<DeferredRevenueSchedule, DbError> {
        let org = parse_uuid(org_id)?;
        let sid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE deferred_revenue_schedules SET status = 'cancelled', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'active'",
        )
        .bind(org)
        .bind(sid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "schedule must be active to cancel".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }
}
