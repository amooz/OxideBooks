use oxidebooks_core::models::{CreateServiceTerritory, ServiceTerritory, UpdateServiceTerritory};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, name, description, region_code, country_code, is_active, \
     created_at, updated_at";

#[derive(sqlx::FromRow)]
struct TerritoryRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    region_code: Option<String>,
    country_code: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TerritoryRow) -> ServiceTerritory {
    ServiceTerritory {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        description: r.description,
        region_code: r.region_code,
        country_code: r.country_code,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct ServiceTerritoryRepo;

impl ServiceTerritoryRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<ServiceTerritory>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<TerritoryRow> = if active_only {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM service_territories \
                 WHERE organization_id = $1 AND is_active = TRUE ORDER BY name"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM service_territories WHERE organization_id = $1 ORDER BY name"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ServiceTerritory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TerritoryRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM service_territories WHERE organization_id = $1 AND id = $2"
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
        input: CreateServiceTerritory,
    ) -> Result<ServiceTerritory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO service_territories \
             (organization_id, name, description, region_code, country_code) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.region_code)
        .bind(&input.country_code)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateServiceTerritory,
    ) -> Result<ServiceTerritory, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        sqlx::query(
            "UPDATE service_territories SET \
             name         = COALESCE($1, name), \
             description  = COALESCE($2, description), \
             region_code  = COALESCE($3, region_code), \
             country_code = COALESCE($4, country_code), \
             is_active    = COALESCE($5, is_active), \
             updated_at   = NOW() \
             WHERE id = $6 AND organization_id = $7",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.region_code)
        .bind(input.country_code)
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
        let rows =
            sqlx::query("DELETE FROM service_territories WHERE id = $1 AND organization_id = $2")
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
