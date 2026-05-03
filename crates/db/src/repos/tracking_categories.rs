use oxidebooks_core::models::{
    CreateTrackingCategory, CreateTrackingOption, TrackingCategory, TrackingOption,
    UpdateTrackingCategory, UpdateTrackingOption,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct OptionRow {
    id: Uuid,
    category_id: Uuid,
    name: String,
    is_active: bool,
    sort_order: i32,
    created_at: OffsetDateTime,
}

fn option_from_row(r: OptionRow) -> TrackingOption {
    TrackingOption {
        id: r.id.to_string(),
        category_id: r.category_id.to_string(),
        name: r.name,
        is_active: r.is_active,
        sort_order: r.sort_order,
        created_at: r.created_at,
    }
}

async fn fetch_options(pool: &PgPool, category_id: Uuid) -> Result<Vec<TrackingOption>, DbError> {
    let rows: Vec<OptionRow> = sqlx::query_as(
        "SELECT id, category_id, name, is_active, sort_order, created_at
         FROM tracking_options WHERE category_id = $1
         ORDER BY sort_order, name",
    )
    .bind(category_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(option_from_row).collect())
}

async fn category_from_row(pool: &PgPool, r: CategoryRow) -> Result<TrackingCategory, DbError> {
    let options = fetch_options(pool, r.id).await?;
    Ok(TrackingCategory {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        is_active: r.is_active,
        options,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

pub struct TrackingCategoryRepo;

impl TrackingCategoryRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<TrackingCategory>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<CategoryRow> = sqlx::query_as(
            "SELECT id, organization_id, name, is_active, created_at, updated_at
             FROM tracking_categories WHERE organization_id = $1 ORDER BY name",
        )
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(category_from_row(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<TrackingCategory, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let row: CategoryRow = sqlx::query_as(
            "SELECT id, organization_id, name, is_active, created_at, updated_at
             FROM tracking_categories WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        category_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateTrackingCategory,
    ) -> Result<TrackingCategory, DbError> {
        let org = parse_uuid(org_id)?;
        let row: CategoryRow = sqlx::query_as(
            "INSERT INTO tracking_categories (organization_id, name)
             VALUES ($1, $2)
             RETURNING id, organization_id, name, is_active, created_at, updated_at",
        )
        .bind(org)
        .bind(&input.name)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        category_from_row(pool, row).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateTrackingCategory,
    ) -> Result<TrackingCategory, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let row: Option<CategoryRow> = sqlx::query_as(
            "UPDATE tracking_categories
             SET name      = COALESCE($3, name),
                 is_active = COALESCE($4, is_active),
                 updated_at = now()
             WHERE organization_id = $1 AND id = $2
             RETURNING id, organization_id, name, is_active, created_at, updated_at",
        )
        .bind(org)
        .bind(cid)
        .bind(&input.name)
        .bind(input.is_active)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        match row {
            Some(r) => category_from_row(pool, r).await,
            None => Err(DbError::NotFound),
        }
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let n =
            sqlx::query("DELETE FROM tracking_categories WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(cid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?
                .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    // ── Options ───────────────────────────────────────────────────────────────

    pub async fn add_option(
        pool: &PgPool,
        org_id: &str,
        category_id: &str,
        input: CreateTrackingOption,
    ) -> Result<TrackingOption, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(category_id)?;
        // Verify category belongs to org
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM tracking_categories WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let row: OptionRow = sqlx::query_as(
            "INSERT INTO tracking_options (category_id, name, sort_order)
             VALUES ($1, $2, $3)
             RETURNING id, category_id, name, is_active, sort_order, created_at",
        )
        .bind(cid)
        .bind(&input.name)
        .bind(input.sort_order)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(option_from_row(row))
    }

    pub async fn update_option(
        pool: &PgPool,
        org_id: &str,
        category_id: &str,
        option_id: &str,
        input: UpdateTrackingOption,
    ) -> Result<TrackingOption, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(category_id)?;
        let oid = parse_uuid(option_id)?;
        // Verify category belongs to org
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM tracking_categories WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let row: Option<OptionRow> = sqlx::query_as(
            "UPDATE tracking_options
             SET name      = COALESCE($3, name),
                 is_active = COALESCE($4, is_active),
                 sort_order = COALESCE($5, sort_order)
             WHERE id = $1 AND category_id = $2
             RETURNING id, category_id, name, is_active, sort_order, created_at",
        )
        .bind(oid)
        .bind(cid)
        .bind(&input.name)
        .bind(input.is_active)
        .bind(input.sort_order)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(option_from_row).ok_or(DbError::NotFound)
    }

    pub async fn delete_option(
        pool: &PgPool,
        org_id: &str,
        category_id: &str,
        option_id: &str,
    ) -> Result<(), DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(category_id)?;
        let oid = parse_uuid(option_id)?;
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM tracking_categories WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let n = sqlx::query("DELETE FROM tracking_options WHERE id = $1 AND category_id = $2")
            .bind(oid)
            .bind(cid)
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
