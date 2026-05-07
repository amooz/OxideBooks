use oxidebooks_core::models::{CreateReportSchedule, ReportSchedule, UpdateReportSchedule};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    report_type: String,
    frequency: String,
    params: serde_json::Value,
    recipients: Vec<String>,
    is_active: bool,
    last_run_at: Option<OffsetDateTime>,
    next_run_at: Option<OffsetDateTime>,
    created_by: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ScheduleRow> for ReportSchedule {
    fn from(r: ScheduleRow) -> Self {
        ReportSchedule {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            report_type: r.report_type,
            frequency: r.frequency,
            params: r.params,
            recipients: r.recipients,
            is_active: r.is_active,
            last_run_at: r.last_run_at,
            next_run_at: r.next_run_at,
            created_by: r.created_by.map(|u| u.to_string()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLS: &str = "id, organization_id, name, report_type, \
    frequency::TEXT, params, recipients, is_active, last_run_at, next_run_at, \
    created_by, created_at, updated_at";

fn add_months(d: time::Date, months: u32) -> time::Date {
    let total = d.month() as u32 - 1 + months;
    let year = d.year() + (total / 12) as i32;
    let month_num = (total % 12 + 1) as u8;
    let month = time::Month::try_from(month_num).unwrap_or(time::Month::January);
    let day = d.day().min(days_in_month(year, month));
    time::Date::from_calendar_date(year, month, day).unwrap_or(d)
}

fn next_run(frequency: &str, from: OffsetDateTime) -> OffsetDateTime {
    use time::Duration;
    let d = from.date();
    match frequency {
        "daily" => from + Duration::days(1),
        "weekly" => from + Duration::weeks(1),
        "quarterly" => from.replace_date(add_months(d, 3)),
        _ => from.replace_date(add_months(d, 1)),
    }
}

fn days_in_month(year: i32, month: time::Month) -> u8 {
    use time::Month::*;
    match month {
        January | March | May | July | August | October | December => 31,
        April | June | September | November => 30,
        February => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

pub struct ReportScheduleRepo;

impl ReportScheduleRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateReportSchedule,
    ) -> Result<ReportSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;

        let valid_frequencies = ["daily", "weekly", "monthly", "quarterly"];
        if !valid_frequencies.contains(&input.frequency.as_str()) {
            return Err(DbError::Conflict(format!(
                "frequency must be one of: {}",
                valid_frequencies.join(", ")
            )));
        }

        let now = OffsetDateTime::now_utc();
        let next_run_at = next_run(&input.frequency, now);

        let row: ScheduleRow = sqlx::query_as(&format!(
            "INSERT INTO report_schedules \
             (organization_id, name, report_type, frequency, params, recipients, \
              next_run_at, created_by) \
             VALUES ($1, $2, $3, $4::report_schedule_frequency, $5, $6, $7, $8) \
             RETURNING {COLS}"
        ))
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.report_type)
        .bind(&input.frequency)
        .bind(&input.params)
        .bind(&input.recipients)
        .bind(next_run_at)
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn get(pool: &PgPool, org_id: &str, id: &str) -> Result<ReportSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let s_uuid = parse_uuid(id)?;

        let row: Option<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM report_schedules \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(s_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(Into::into).ok_or(DbError::NotFound)
    }

    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<ReportSchedule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM report_schedules \
             WHERE organization_id = $1 AND ($2 = FALSE OR is_active = TRUE) \
             ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .bind(active_only)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateReportSchedule,
    ) -> Result<ReportSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let s_uuid = parse_uuid(id)?;

        let existing = Self::get(pool, org_id, id).await?;

        let name = input.name.unwrap_or(existing.name);
        let frequency = input.frequency.unwrap_or(existing.frequency);
        let params = input.params.unwrap_or(existing.params);
        let recipients = input.recipients.unwrap_or(existing.recipients);
        let is_active = input.is_active.unwrap_or(existing.is_active);

        let valid_frequencies = ["daily", "weekly", "monthly", "quarterly"];
        if !valid_frequencies.contains(&frequency.as_str()) {
            return Err(DbError::Conflict(format!(
                "frequency must be one of: {}",
                valid_frequencies.join(", ")
            )));
        }

        let row: ScheduleRow = sqlx::query_as(&format!(
            "UPDATE report_schedules SET \
             name = $3, frequency = $4::report_schedule_frequency, params = $5, \
             recipients = $6, is_active = $7, updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 \
             RETURNING {COLS}"
        ))
        .bind(s_uuid)
        .bind(org_uuid)
        .bind(&name)
        .bind(&frequency)
        .bind(&params)
        .bind(&recipients)
        .bind(is_active)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let s_uuid = parse_uuid(id)?;

        let result =
            sqlx::query("DELETE FROM report_schedules WHERE id = $1 AND organization_id = $2")
                .bind(s_uuid)
                .bind(org_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn mark_run(pool: &PgPool, id: &str) -> Result<(), DbError> {
        let s_uuid = parse_uuid(id)?;

        let freq: Option<(String,)> =
            sqlx::query_as("SELECT frequency::TEXT FROM report_schedules WHERE id = $1")
                .bind(s_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        let (frequency,) = freq.ok_or(DbError::NotFound)?;
        let now = OffsetDateTime::now_utc();
        let next = next_run(&frequency, now);

        sqlx::query(
            "UPDATE report_schedules SET last_run_at = $2, next_run_at = $3, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(s_uuid)
        .bind(now)
        .bind(next)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}
