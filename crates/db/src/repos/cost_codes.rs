use oxidebooks_core::models::{CostCode, CreateCostCode, UpdateCostCode};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct CostCodeRow {
    id: Uuid,
    organization_id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    cost_type: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

const COLS: &str =
    "id, organization_id, code, name, description, cost_type, is_active, created_at, updated_at";

fn from_row(r: CostCodeRow) -> CostCode {
    CostCode {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        code: r.code,
        name: r.name,
        description: r.description,
        cost_type: r.cost_type,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct CostCodeRepo;

impl CostCodeRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<CostCode>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<CostCodeRow> = if active_only {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM cost_codes \
                 WHERE organization_id = $1 AND is_active = TRUE \
                 ORDER BY code ASC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM cost_codes \
                 WHERE organization_id = $1 ORDER BY code ASC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<CostCode, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: CostCodeRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM cost_codes WHERE organization_id = $1 AND id = $2"
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
        input: CreateCostCode,
    ) -> Result<CostCode, DbError> {
        validate_cost_type(&input.cost_type)?;
        let org_uuid = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO cost_codes (organization_id, code, name, description, cost_type) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.cost_type)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateCostCode,
    ) -> Result<CostCode, DbError> {
        if let Some(ref ct) = input.cost_type {
            validate_cost_type(ct)?;
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE cost_codes SET \
             name        = COALESCE($3, name), \
             description = COALESCE($4, description), \
             cost_type   = COALESCE($5, cost_type), \
             is_active   = COALESCE($6, is_active), \
             updated_at  = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.cost_type)
        .bind(input.is_active)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query("DELETE FROM cost_codes WHERE organization_id = $1 AND id = $2")
            .bind(org_uuid)
            .bind(id_uuid)
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

fn validate_cost_type(ct: &str) -> Result<(), DbError> {
    match ct {
        "labor" | "material" | "equipment" | "subcontractor" | "overhead" | "other" => Ok(()),
        other => Err(DbError::Conflict(format!("invalid cost_type '{other}'"))),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
