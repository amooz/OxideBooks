use oxidebooks_core::models::{CreateTaxGroup, TaxGroup, TaxGroupRate, UpdateTaxGroup};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct GroupRateRow {
    id: Uuid,
    group_id: Uuid,
    tax_rate_id: Uuid,
    tax_rate_name: String,
    rate_bps: i32,
    sort_order: i32,
}

async fn fetch_rates(pool: &PgPool, group_id: Uuid) -> Result<Vec<TaxGroupRate>, DbError> {
    let rows: Vec<GroupRateRow> = sqlx::query_as(
        "SELECT tgr.id, tgr.group_id, tgr.tax_rate_id, tr.name AS tax_rate_name,
                tr.rate_bps, tgr.sort_order
         FROM tax_group_rates tgr
         JOIN tax_rates tr ON tr.id = tgr.tax_rate_id
         WHERE tgr.group_id = $1
         ORDER BY tgr.sort_order, tgr.id",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| TaxGroupRate {
            id: r.id.to_string(),
            group_id: r.group_id.to_string(),
            tax_rate_id: r.tax_rate_id.to_string(),
            tax_rate_name: r.tax_rate_name,
            rate: r.rate_bps as i64,
            sort_order: r.sort_order,
        })
        .collect())
}

async fn group_from_parts(pool: &PgPool, r: GroupRow) -> Result<TaxGroup, DbError> {
    let rates = fetch_rates(pool, r.id).await?;
    let combined_rate: i64 = rates.iter().map(|r| r.rate).sum();
    Ok(TaxGroup {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        combined_rate,
        is_active: r.is_active,
        rates,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const COLS: &str = "id, organization_id, name, description, is_active, created_at, updated_at";

pub struct TaxGroupRepo;

impl TaxGroupRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<TaxGroup>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<GroupRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_groups WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(group_from_parts(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<TaxGroup, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(id)?;
        let row: GroupRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM tax_groups WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        group_from_parts(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateTaxGroup,
    ) -> Result<TaxGroup, DbError> {
        let org = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO tax_groups (id, organization_id, name, description)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(org)
        .bind(&input.name)
        .bind(&input.description)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, rate_input) in input.rates.iter().enumerate() {
            let rate_id = parse_uuid(&rate_input.tax_rate_id)?;
            // Verify the tax_rate belongs to this org
            let exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM tax_rates WHERE organization_id = $1 AND id = $2")
                    .bind(org)
                    .bind(rate_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_sqlx_err)?;
            if exists.is_none() {
                return Err(DbError::NotFound);
            }
            sqlx::query(
                "INSERT INTO tax_group_rates (group_id, tax_rate_id, sort_order)
                 VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(rate_id)
            .bind(rate_input.sort_order.max(i as i32))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTaxGroup,
    ) -> Result<TaxGroup, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let n = sqlx::query(
            "UPDATE tax_groups
             SET name        = COALESCE($3, name),
                 description = COALESCE($4, description),
                 is_active   = COALESCE($5, is_active),
                 updated_at  = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(gid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.is_active)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }

        if let Some(rates) = &input.rates {
            sqlx::query("DELETE FROM tax_group_rates WHERE group_id = $1")
                .bind(gid)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;

            for (i, rate_input) in rates.iter().enumerate() {
                let rate_id = parse_uuid(&rate_input.tax_rate_id)?;
                let exists: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM tax_rates WHERE organization_id = $1 AND id = $2",
                )
                .bind(org)
                .bind(rate_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
                if exists.is_none() {
                    return Err(DbError::NotFound);
                }
                sqlx::query(
                    "INSERT INTO tax_group_rates (group_id, tax_rate_id, sort_order)
                     VALUES ($1, $2, $3)",
                )
                .bind(gid)
                .bind(rate_id)
                .bind(rate_input.sort_order.max(i as i32))
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
            }
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM tax_groups WHERE organization_id = $1 AND id = $2")
            .bind(org)
            .bind(gid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}
