use oxidebooks_core::models::{
    CreateRecurringSchedule, Frequency, RecurringSchedule, UpdateRecurringSchedule,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    id: Uuid,
    organization_id: Uuid,
    template: serde_json::Value,
    frequency: String,
    interval_count: i32,
    next_due_date: Date,
    end_date: Option<Date>,
    auto_send: bool,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ScheduleRow) -> RecurringSchedule {
    RecurringSchedule {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        template: r.template,
        frequency: Frequency::from_str(&r.frequency).unwrap_or(Frequency::Monthly),
        interval_count: r.interval_count,
        next_due_date: r.next_due_date,
        end_date: r.end_date,
        auto_send: r.auto_send,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, template, frequency, interval_count, next_due_date, \
                    end_date, auto_send, is_active, created_at, updated_at";

pub struct RecurringRepo;

impl RecurringRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<RecurringSchedule>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM recurring_schedules WHERE organization_id = $1 \
             ORDER BY next_due_date ASC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<RecurringSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ScheduleRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM recurring_schedules WHERE organization_id = $1 AND id = $2"
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
        input: CreateRecurringSchedule,
    ) -> Result<RecurringSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO recurring_schedules \
             (organization_id, template, frequency, interval_count, next_due_date, end_date, auto_send) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.template)
        .bind(input.frequency.to_string())
        .bind(input.interval_count)
        .bind(input.next_due_date)
        .bind(input.end_date)
        .bind(input.auto_send)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateRecurringSchedule,
    ) -> Result<RecurringSchedule, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        // Ensure exists
        Self::get_by_id(pool, org_id, id).await?;

        sqlx::query(
            "UPDATE recurring_schedules SET \
             next_due_date = COALESCE($1, next_due_date), \
             end_date      = COALESCE($2, end_date), \
             auto_send     = COALESCE($3, auto_send), \
             is_active     = COALESCE($4, is_active), \
             updated_at    = NOW() \
             WHERE id = $5 AND organization_id = $6",
        )
        .bind(input.next_due_date)
        .bind(input.end_date)
        .bind(input.auto_send)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n =
            sqlx::query("DELETE FROM recurring_schedules WHERE id = $1 AND organization_id = $2")
                .bind(id_uuid)
                .bind(org_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Returns all active schedules due on or before `today` across all orgs.
    /// Used by the background scheduler.
    pub async fn list_due(pool: &PgPool, today: Date) -> Result<Vec<RecurringSchedule>, DbError> {
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM recurring_schedules \
             WHERE is_active = TRUE AND next_due_date <= $1 \
               AND (end_date IS NULL OR end_date >= $1) \
             ORDER BY next_due_date ASC"
        ))
        .bind(today)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Advance `next_due_date` by the schedule's frequency × interval_count.
    pub async fn advance(pool: &PgPool, id: &str, new_due: Date) -> Result<(), DbError> {
        let id_uuid = parse_uuid(id)?;
        sqlx::query(
            "UPDATE recurring_schedules SET next_due_date = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(new_due)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
