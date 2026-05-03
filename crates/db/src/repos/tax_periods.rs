use oxidebooks_core::models::{CreateTaxPeriod, FileTaxPeriod, TaxPeriod};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, name, period_start, period_end, \
     tax_collected, tax_paid, net_tax, status, filed_at, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct TaxPeriodRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    period_start: Date,
    period_end: Date,
    tax_collected: i64,
    tax_paid: i64,
    net_tax: i64,
    status: String,
    filed_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<TaxPeriodRow> for TaxPeriod {
    fn from(r: TaxPeriodRow) -> Self {
        TaxPeriod {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            name: r.name,
            period_start: r.period_start,
            period_end: r.period_end,
            tax_collected: r.tax_collected,
            tax_paid: r.tax_paid,
            net_tax: r.net_tax,
            status: r.status,
            filed_at: r.filed_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct TaxPeriodRepo;

impl TaxPeriodRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<TaxPeriod>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TaxPeriodRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_periods \
             WHERE organization_id = $1 ORDER BY period_start DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(TaxPeriod::from).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxPeriod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaxPeriodRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_periods WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(row.into())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateTaxPeriod,
    ) -> Result<TaxPeriod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        if input.period_end < input.period_start {
            return Err(DbError::Conflict(
                "period_end must be >= period_start".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tax_periods (id, organization_id, name, period_start, period_end) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.period_start)
        .bind(input.period_end)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        let row: TaxPeriodRow =
            sqlx::query_as(&format!("SELECT {COLS} FROM tax_periods WHERE id = $1"))
                .bind(id)
                .fetch_one(pool)
                .await
                .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn file(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: FileTaxPeriod,
    ) -> Result<TaxPeriod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let net = input.tax_collected - input.tax_paid;
        let now = time::OffsetDateTime::now_utc();
        let rows = sqlx::query(
            "UPDATE tax_periods SET \
             tax_collected = $3, tax_paid = $4, net_tax = $5, \
             status = 'filed', filed_at = $6, updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(input.tax_collected)
        .bind(input.tax_paid)
        .bind(net)
        .bind(now)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "period not found or not in open status".into(),
            ));
        }
        let row: TaxPeriodRow =
            sqlx::query_as(&format!("SELECT {COLS} FROM tax_periods WHERE id = $1"))
                .bind(id_uuid)
                .fetch_one(pool)
                .await
                .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn lock(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxPeriod, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE tax_periods SET status = 'locked', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'filed'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "period not found or not in filed status".into(),
            ));
        }
        let row: TaxPeriodRow =
            sqlx::query_as(&format!("SELECT {COLS} FROM tax_periods WHERE id = $1"))
                .bind(id_uuid)
                .fetch_one(pool)
                .await
                .map_err(map_sqlx_err)?;
        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "DELETE FROM tax_periods \
             WHERE organization_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "period not found or not deletable (only open periods can be deleted)".into(),
            ));
        }
        Ok(())
    }
}
