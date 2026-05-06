use oxidebooks_core::models::{CreateProjectTask, ProjectTask, UpdateProjectTask};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    organization_id: Uuid,
    project_id: Uuid,
    phase_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    assignee_id: Option<Uuid>,
    status: String,
    due_date: Option<Date>,
    estimated_minutes: Option<i32>,
    actual_minutes: i32,
    sort_order: i32,
    completed_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: TaskRow) -> ProjectTask {
    ProjectTask {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        project_id: r.project_id.to_string(),
        phase_id: r.phase_id.map(|u| u.to_string()),
        name: r.name,
        description: r.description,
        assignee_id: r.assignee_id.map(|u| u.to_string()),
        status: r.status,
        due_date: r.due_date,
        estimated_minutes: r.estimated_minutes,
        actual_minutes: r.actual_minutes,
        sort_order: r.sort_order,
        completed_at: r.completed_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str =
    "id, organization_id, project_id, phase_id, name, description, assignee_id, status, \
     due_date, estimated_minutes, actual_minutes, sort_order, completed_at, \
     created_at, updated_at";

pub struct ProjectTaskRepo;

impl ProjectTaskRepo {
    pub async fn list_for_project(
        pool: &PgPool,
        org_id: &str,
        project_id: &str,
    ) -> Result<Vec<ProjectTask>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let proj_uuid = parse_uuid(project_id)?;
        let rows: Vec<TaskRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_tasks \
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

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<ProjectTask, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: TaskRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM project_tasks WHERE organization_id = $1 AND id = $2"
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
        input: CreateProjectTask,
    ) -> Result<ProjectTask, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let proj_uuid = parse_uuid(project_id)?;
        let phase_uuid = input.phase_id.as_deref().map(parse_uuid).transpose()?;
        let assignee_uuid = input.assignee_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO project_tasks \
             (organization_id, project_id, phase_id, name, description, assignee_id, \
              due_date, estimated_minutes, sort_order) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(proj_uuid)
        .bind(phase_uuid)
        .bind(&input.name)
        .bind(&input.description)
        .bind(assignee_uuid)
        .bind(input.due_date)
        .bind(input.estimated_minutes)
        .bind(input.sort_order)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateProjectTask,
    ) -> Result<ProjectTask, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let phase_uuid = input.phase_id.as_deref().map(parse_uuid).transpose()?;
        let assignee_uuid = input.assignee_id.as_deref().map(parse_uuid).transpose()?;

        let completed_at: Option<OffsetDateTime> = match input.status.as_deref() {
            Some("completed") => Some(OffsetDateTime::now_utc()),
            _ => None,
        };

        sqlx::query(
            "UPDATE project_tasks SET \
             name               = COALESCE($1, name), \
             description        = COALESCE($2, description), \
             phase_id           = COALESCE($3, phase_id), \
             assignee_id        = COALESCE($4, assignee_id), \
             status             = COALESCE($5, status), \
             due_date           = COALESCE($6, due_date), \
             estimated_minutes  = COALESCE($7, estimated_minutes), \
             actual_minutes     = COALESCE($8, actual_minutes), \
             sort_order         = COALESCE($9, sort_order), \
             completed_at       = CASE WHEN $5 = 'completed' THEN $10 ELSE completed_at END, \
             updated_at         = NOW() \
             WHERE organization_id = $11 AND id = $12",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(phase_uuid)
        .bind(assignee_uuid)
        .bind(&input.status)
        .bind(input.due_date)
        .bind(input.estimated_minutes)
        .bind(input.actual_minutes)
        .bind(input.sort_order)
        .bind(completed_at)
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
        let rows = sqlx::query("DELETE FROM project_tasks WHERE organization_id = $1 AND id = $2")
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
