use oxidebooks_core::models::{CreateRevRecSchedule, RecognizeRevRec, RevRecEntry, RevRecSchedule};
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
    revenue_account_id: Option<Uuid>,
    deferred_account_id: Option<Uuid>,
    description: String,
    method: String,
    total_amount: i64,
    recognized_amount: i64,
    start_date: Date,
    end_date: Date,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    schedule_id: Uuid,
    organization_id: Uuid,
    period: Date,
    amount: i64,
    journal_entry_id: Option<Uuid>,
    posted_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl From<EntryRow> for RevRecEntry {
    fn from(r: EntryRow) -> Self {
        RevRecEntry {
            id: r.id.to_string(),
            schedule_id: r.schedule_id.to_string(),
            organization_id: r.organization_id.to_string(),
            period: r.period,
            amount: r.amount,
            journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
            posted_at: r.posted_at,
            created_at: r.created_at,
        }
    }
}

const SCHED_COLS: &str = "id, organization_id, invoice_id, revenue_account_id, \
     deferred_account_id, description, method::TEXT, total_amount, recognized_amount, \
     start_date, end_date, status::TEXT, created_at, updated_at";

const ENTRY_COLS: &str =
    "id, schedule_id, organization_id, period, amount, journal_entry_id, posted_at, created_at";

async fn fetch_entries(pool: &PgPool, schedule_id: Uuid) -> Result<Vec<RevRecEntry>, DbError> {
    let rows: Vec<EntryRow> = sqlx::query_as(&format!(
        "SELECT {ENTRY_COLS} FROM rev_rec_entries \
         WHERE schedule_id = $1 ORDER BY period ASC"
    ))
    .bind(schedule_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(RevRecEntry::from).collect())
}

fn to_schedule(r: ScheduleRow, entries: Vec<RevRecEntry>) -> RevRecSchedule {
    let remaining = r.total_amount - r.recognized_amount;
    RevRecSchedule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        revenue_account_id: r.revenue_account_id.map(|u| u.to_string()),
        deferred_account_id: r.deferred_account_id.map(|u| u.to_string()),
        description: r.description,
        method: r.method,
        total_amount: r.total_amount,
        recognized_amount: r.recognized_amount,
        remaining_amount: remaining,
        start_date: r.start_date,
        end_date: r.end_date,
        status: r.status,
        entries,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

/// Generate straight-line monthly periods between start_date and end_date.
fn generate_straight_line_periods(start: Date, end: Date, total: i64) -> Vec<(Date, i64)> {
    let mut periods: Vec<Date> = Vec::new();
    let mut current = Date::from_calendar_date(start.year(), start.month(), 1).unwrap_or(start);
    let end_month = Date::from_calendar_date(end.year(), end.month(), 1).unwrap_or(end);

    while current <= end_month {
        periods.push(current);
        // Advance to next month
        let (y, m) = if current.month() == time::Month::December {
            (current.year() + 1, time::Month::January)
        } else {
            (current.year(), current.month().next())
        };
        current = Date::from_calendar_date(y, m, 1).unwrap_or(current);
        // Safety: break if we somehow loop infinitely
        if periods.len() > 600 {
            break;
        }
    }

    if periods.is_empty() {
        return vec![(start, total)];
    }

    let n = periods.len() as i64;
    let base = total / n;
    let remainder = total % n;

    periods
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let amt = if i == 0 { base + remainder } else { base };
            (p, amt)
        })
        .filter(|(_, amt)| *amt > 0)
        .collect()
}

pub struct RevRecRepo;

impl RevRecRepo {
    pub async fn create_for_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        input: CreateRevRecSchedule,
    ) -> Result<RevRecSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        // Verify invoice belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM invoices WHERE id = $1 AND organization_id = $2")
                .bind(inv_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rev_acct = input
            .revenue_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;
        let def_acct = input
            .deferred_account_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        let method = if matches!(
            input.method.as_str(),
            "straight_line" | "milestone" | "usage_based" | "manual"
        ) {
            input.method.clone()
        } else {
            return Err(DbError::Conflict(format!(
                "invalid rev-rec method '{}'",
                input.method
            )));
        };

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO rev_rec_schedules \
             (id, organization_id, invoice_id, revenue_account_id, deferred_account_id, \
              description, method, total_amount, start_date, end_date) \
             VALUES ($1, $2, $3, $4, $5, $6, $7::rev_rec_method, $8, $9, $10)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(rev_acct)
        .bind(def_acct)
        .bind(&input.description)
        .bind(&method)
        .bind(input.total_amount)
        .bind(input.start_date)
        .bind(input.end_date)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Generate entries for straight_line method.
        if method == "straight_line" {
            let periods = generate_straight_line_periods(
                input.start_date,
                input.end_date,
                input.total_amount,
            );
            for (period, amount) in periods {
                sqlx::query(
                    "INSERT INTO rev_rec_entries (schedule_id, organization_id, period, amount) \
                     VALUES ($1, $2, $3, $4) ON CONFLICT (schedule_id, period) DO NOTHING",
                )
                .bind(id)
                .bind(org_uuid)
                .bind(period)
                .bind(amount)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
            }
        }

        Self::get(pool, org_id, &id.to_string()).await
    }

    pub async fn get(pool: &PgPool, org_id: &str, id: &str) -> Result<RevRecSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let s_uuid = parse_uuid(id)?;
        let row: ScheduleRow = sqlx::query_as(&format!(
            "SELECT {SCHED_COLS} FROM rev_rec_schedules \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(s_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let entries = fetch_entries(pool, s_uuid).await?;
        Ok(to_schedule(row, entries))
    }

    pub async fn get_for_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<RevRecSchedule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {SCHED_COLS} FROM rev_rec_schedules \
             WHERE invoice_id = $1 AND organization_id = $2 ORDER BY created_at ASC"
        ))
        .bind(inv_uuid)
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut schedules = Vec::with_capacity(rows.len());
        for row in rows {
            let entries = fetch_entries(pool, row.id).await?;
            schedules.push(to_schedule(row, entries));
        }
        Ok(schedules)
    }

    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<RevRecSchedule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {SCHED_COLS} FROM rev_rec_schedules \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut schedules = Vec::with_capacity(rows.len());
        for row in rows {
            let entries = fetch_entries(pool, row.id).await?;
            schedules.push(to_schedule(row, entries));
        }
        Ok(schedules)
    }

    /// Recognize revenue for a period (YYYY-MM). Posts pending entries and updates
    /// recognized_amount on the schedule.
    pub async fn recognize(
        pool: &PgPool,
        org_id: &str,
        input: RecognizeRevRec,
    ) -> Result<serde_json::Value, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        // Parse period "YYYY-MM" into the first day of that month.
        let period_date = {
            let padded = format!("{}-01", input.period.trim());
            let fmt = time::macros::format_description!("[year]-[month]-[day]");
            Date::parse(&padded, fmt).map_err(|_| {
                DbError::Conflict(format!(
                    "invalid period '{}'; expected YYYY-MM",
                    input.period
                ))
            })?
        };

        let schedule_filter = input.schedule_id.as_deref().map(parse_uuid).transpose()?;

        // Fetch unposted entries for this period.
        let entries: Vec<(Uuid, Uuid, i64)> = {
            #[derive(sqlx::FromRow)]
            struct E {
                id: Uuid,
                schedule_id: Uuid,
                amount: i64,
            }
            let rows: Vec<E> = if let Some(s_uuid) = schedule_filter {
                sqlx::query_as(
                    "SELECT id, schedule_id, amount FROM rev_rec_entries \
                     WHERE organization_id = $1 AND period = $2 AND posted_at IS NULL \
                       AND schedule_id = $3",
                )
                .bind(org_uuid)
                .bind(period_date)
                .bind(s_uuid)
                .fetch_all(pool)
                .await
                .map_err(map_sqlx_err)?
            } else {
                sqlx::query_as(
                    "SELECT id, schedule_id, amount FROM rev_rec_entries \
                     WHERE organization_id = $1 AND period = $2 AND posted_at IS NULL",
                )
                .bind(org_uuid)
                .bind(period_date)
                .fetch_all(pool)
                .await
                .map_err(map_sqlx_err)?
            };
            rows.into_iter()
                .map(|r| (r.id, r.schedule_id, r.amount))
                .collect()
        };

        if entries.is_empty() {
            return Err(DbError::Conflict(
                "no unposted entries found for this period".into(),
            ));
        }

        let mut recognized_count = 0usize;
        let mut total_recognized: i64 = 0;

        for (entry_id, schedule_id, amount) in &entries {
            // Mark entry as posted.
            sqlx::query("UPDATE rev_rec_entries SET posted_at = NOW() WHERE id = $1")
                .bind(entry_id)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;

            // Update schedule recognized_amount.
            sqlx::query(
                "UPDATE rev_rec_schedules \
                 SET recognized_amount = recognized_amount + $1, updated_at = NOW() \
                 WHERE id = $2",
            )
            .bind(amount)
            .bind(schedule_id)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

            // Mark schedule completed if fully recognized.
            sqlx::query(
                "UPDATE rev_rec_schedules SET status = 'completed', updated_at = NOW() \
                 WHERE id = $1 AND recognized_amount >= total_amount AND status = 'active'",
            )
            .bind(schedule_id)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

            recognized_count += 1;
            total_recognized += amount;
        }

        Ok(serde_json::json!({
            "period": input.period,
            "entries_recognized": recognized_count,
            "total_amount": total_recognized,
        }))
    }
}
