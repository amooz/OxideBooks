use oxidebooks_core::models::{CreateProjectPhase, ProjectPhase, UpdateProjectPhase};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PhaseRow {
    id: Uuid,
    organization_id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    budget: i64,
    start_date: Option<Date>,
    end_date: Option<Date>,
    status: String,
    sort_order: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: PhaseRow) -> ProjectPhase {
    ProjectPhase {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        project_id: r.project_id.to_string(),
        name: r.name,
        description: r.description,
        budget: r.budget,
        start_date: r.start_date,
        end_date: r.end_date,
        status: r.status,
        sort_order: r.sort_order,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, project_id, name, description, budget, \
                    start_date, end_date, status, sort_order, created_at, updated_at";

pub struct ProjectPhaseRepo;

impl ProjectPhaseRepo {
    pub async fn list_for_project(
        pool: &PgPool,
        org_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectPhase>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let proj_uuid = parse_uuid(project_id)?;
        let rows: Vec<PhaseRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_phases \
             WHERE organization_id = $1 AND project_id = $2 \
             ORDER BY sort_order ASC, created_at ASC"
        ))
        .bind(org_uuid)
        .bind(proj_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<ProjectPhase, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: PhaseRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_phases WHERE organization_id = $1 AND id = $2"
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
        project_id: &str,
        input: CreateProjectPhase,
    ) -> Result<ProjectPhase, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let proj_uuid = parse_uuid(project_id)?;
        let budget = input.budget.unwrap_or(0);
        let sort_order = input.sort_order.unwrap_or(0);

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO project_phases \
             (organization_id, project_id, name, description, budget, start_date, end_date, \
              sort_order) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(proj_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(budget)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(sort_order)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateProjectPhase,
    ) -> Result<ProjectPhase, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        sqlx::query(
            "UPDATE project_phases SET \
             name        = COALESCE($1, name), \
             description = COALESCE($2, description), \
             budget      = COALESCE($3, budget), \
             start_date  = COALESCE($4, start_date), \
             end_date    = COALESCE($5, end_date), \
             status      = COALESCE($6, status), \
             sort_order  = COALESCE($7, sort_order), \
             updated_at  = NOW() \
             WHERE organization_id = $8 AND id = $9",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.budget)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(input.status)
        .bind(input.sort_order)
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query("DELETE FROM project_phases WHERE organization_id = $1 AND id = $2")
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

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
