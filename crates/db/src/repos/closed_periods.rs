use oxidebooks_core::models::{ClosedPeriod, CreateClosedPeriod};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ClosedPeriodRow {
    id: Uuid,
    organization_id: Uuid,
    period_start: Date,
    period_end: Date,
    closed_by: Option<Uuid>,
    notes: Option<String>,
    closed_at: OffsetDateTime,
}

fn from_row(r: ClosedPeriodRow) -> ClosedPeriod {
    ClosedPeriod {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        period_start: r.period_start,
        period_end: r.period_end,
        closed_by: r.closed_by.map(|u| u.to_string()),
        notes: r.notes,
        closed_at: r.closed_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct ClosedPeriodRepo;

impl ClosedPeriodRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ClosedPeriod>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ClosedPeriodRow> = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, closed_by, notes, closed_at \
             FROM closed_periods WHERE organization_id = $1 ORDER BY period_start DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn close(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateClosedPeriod,
    ) -> Result<ClosedPeriod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO closed_periods (organization_id, period_start, period_end, closed_by, notes) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(user_uuid)
        .bind(input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: ClosedPeriodRow = sqlx::query_as(
            "SELECT id, organization_id, period_start, period_end, closed_by, notes, closed_at \
             FROM closed_periods WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn reopen(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM closed_periods WHERE id = $1 AND organization_id = $2")
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

    /// Returns true if `date` falls within any closed period for the org.
    pub async fn is_date_closed(pool: &PgPool, org_id: &str, date: Date) -> Result<bool, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let closed: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM closed_periods \
               WHERE organization_id = $1 \
                 AND $2 BETWEEN period_start AND period_end \
             )",
        )
        .bind(org_uuid)
        .bind(date)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(closed)
    }
}
