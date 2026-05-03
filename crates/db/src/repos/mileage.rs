use oxidebooks_core::models::{CreateMileageTrip, MileageSummary, MileageTrip};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TripRow {
    id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    trip_date: Date,
    distance_km: f64,
    purpose: String,
    rate_per_km: i64,
    reimbursable: bool,
    expense_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

fn from_row(r: TripRow) -> MileageTrip {
    MileageTrip {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        user_id: r.user_id.to_string(),
        trip_date: r.trip_date,
        distance_km: r.distance_km,
        purpose: r.purpose,
        rate_per_km: r.rate_per_km,
        reimbursable: r.reimbursable,
        expense_id: r.expense_id.map(|u| u.to_string()),
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct MileageRepo;

impl MileageRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<MileageTrip>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;
        let rows: Vec<TripRow> = if let Some(uid) = user_uuid {
            sqlx::query_as(
                "SELECT id, organization_id, user_id, trip_date, distance_km, purpose, \
                 rate_per_km, reimbursable, expense_id, created_at \
                 FROM mileage_trips WHERE organization_id = $1 AND user_id = $2 \
                 ORDER BY trip_date DESC",
            )
            .bind(org_uuid)
            .bind(uid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, user_id, trip_date, distance_km, purpose, \
                 rate_per_km, reimbursable, expense_id, created_at \
                 FROM mileage_trips WHERE organization_id = $1 \
                 ORDER BY trip_date DESC",
            )
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateMileageTrip,
    ) -> Result<MileageTrip, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO mileage_trips \
             (organization_id, user_id, trip_date, distance_km, purpose, rate_per_km, reimbursable) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(user_uuid)
        .bind(input.trip_date)
        .bind(input.distance_km)
        .bind(&input.purpose)
        .bind(input.rate_per_km)
        .bind(input.reimbursable)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: TripRow = sqlx::query_as(
            "SELECT id, organization_id, user_id, trip_date, distance_km, purpose, \
             rate_per_km, reimbursable, expense_id, created_at \
             FROM mileage_trips WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(from_row(row))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM mileage_trips WHERE id = $1 AND organization_id = $2")
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

    pub async fn summary(
        pool: &PgPool,
        org_id: &str,
        user_id: Option<&str>,
    ) -> Result<MileageSummary, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = user_id.map(parse_uuid).transpose()?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            total_km: Option<f64>,
            total_reimbursable: Option<f64>,
            trip_count: Option<i64>,
        }

        let row: SummaryRow = if let Some(uid) = user_uuid {
            sqlx::query_as(
                "SELECT SUM(distance_km) AS total_km, \
                 SUM(distance_km * rate_per_km) AS total_reimbursable, \
                 COUNT(*) AS trip_count \
                 FROM mileage_trips WHERE organization_id = $1 AND user_id = $2 AND reimbursable = true",
            )
            .bind(org_uuid)
            .bind(uid)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT SUM(distance_km) AS total_km, \
                 SUM(distance_km * rate_per_km) AS total_reimbursable, \
                 COUNT(*) AS trip_count \
                 FROM mileage_trips WHERE organization_id = $1 AND reimbursable = true",
            )
            .bind(org_uuid)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        Ok(MileageSummary {
            total_km: row.total_km.unwrap_or(0.0),
            total_reimbursable: row.total_reimbursable.unwrap_or(0.0).round() as i64,
            trip_count: row.trip_count.unwrap_or(0),
        })
    }
}
