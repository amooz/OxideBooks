use oxidebooks_core::models::{CreateTaxRate, TaxRate, UpdateTaxRate};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TaxRateRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    rate_bps: i32,
    tax_type: String,
    is_default: bool,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TaxRateRow) -> TaxRate {
    TaxRate {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        rate_bps: r.rate_bps,
        tax_type: r.tax_type,
        is_default: r.is_default,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct TaxRateRepo;

impl TaxRateRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<TaxRate>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TaxRateRow> = sqlx::query_as(
            "SELECT id, organization_id, name, rate_bps, tax_type, is_default, is_active, \
             created_at, updated_at FROM tax_rates WHERE organization_id = $1 ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxRate, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaxRateRow = sqlx::query_as(
            "SELECT id, organization_id, name, rate_bps, tax_type, is_default, is_active, \
             created_at, updated_at FROM tax_rates WHERE organization_id = $1 AND id = $2",
        )
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
        input: CreateTaxRate,
    ) -> Result<TaxRate, DbError> {
        if input.rate_bps < 0 || input.rate_bps > 100_000 {
            return Err(DbError::Conflict(
                "rate_bps must be between 0 and 100000".into(),
            ));
        }
        let org_uuid = parse_uuid(org_id)?;
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        if input.is_default {
            sqlx::query(
                "UPDATE tax_rates SET is_default = FALSE, updated_at = NOW() \
                 WHERE organization_id = $1 AND is_default = TRUE",
            )
            .bind(org_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO tax_rates \
             (organization_id, name, rate_bps, tax_type, is_default) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(input.rate_bps)
        .bind(&input.tax_type)
        .bind(input.is_default)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTaxRate,
    ) -> Result<TaxRate, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Verify exists
        Self::get_by_id(pool, org_id, id).await?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        if let Some(true) = input.is_default {
            sqlx::query(
                "UPDATE tax_rates SET is_default = FALSE, updated_at = NOW() \
                 WHERE organization_id = $1 AND is_default = TRUE AND id != $2",
            )
            .bind(org_uuid)
            .bind(id_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query(
            "UPDATE tax_rates SET \
             name       = COALESCE($1, name), \
             rate_bps   = COALESCE($2, rate_bps), \
             tax_type   = COALESCE($3, tax_type), \
             is_default = COALESCE($4, is_default), \
             is_active  = COALESCE($5, is_active), \
             updated_at = NOW() \
             WHERE id = $6 AND organization_id = $7",
        )
        .bind(input.name)
        .bind(input.rate_bps)
        .bind(input.tax_type)
        .bind(input.is_default)
        .bind(input.is_active)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query("DELETE FROM tax_rates WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
