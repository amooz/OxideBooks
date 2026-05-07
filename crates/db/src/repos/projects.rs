use oxidebooks_core::models::{CreateProject, Project, ProjectSummary, UpdateProject};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    contact_id: Option<Uuid>,
    status: String,
    billing_method: String,
    budget_amount: Option<i64>,
    start_date: Option<Date>,
    end_date: Option<Date>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    project_id: Uuid,
    project_name: String,
    total_invoiced: i64,
    total_expenses: i64,
    total_time_cost: i64,
    net: i64,
}

fn from_row(r: ProjectRow) -> Project {
    Project {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        contact_id: r.contact_id.map(|u| u.to_string()),
        status: r.status,
        billing_method: r.billing_method,
        budget_amount: r.budget_amount,
        start_date: r.start_date,
        end_date: r.end_date,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const COLS: &str = "id, organization_id, name, contact_id, status, billing_method, \
                    budget_amount, start_date, end_date, notes, created_at, updated_at";

pub struct ProjectRepo;

impl ProjectRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<Project>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ProjectRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM projects WHERE organization_id = $1 AND status = $2 \
                 ORDER BY name"
            ))
            .bind(org_uuid)
            .bind(s)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM projects WHERE organization_id = $1 ORDER BY name"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Project, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ProjectRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM projects WHERE organization_id = $1 AND id = $2"
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
        input: CreateProject,
    ) -> Result<Project, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO projects \
             (organization_id, name, contact_id, status, billing_method, \
              budget_amount, start_date, end_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&input.name)
        .bind(contact_uuid)
        .bind(&input.status)
        .bind(&input.billing_method)
        .bind(input.budget_amount)
        .bind(input.start_date)
        .bind(input.end_date)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateProject,
    ) -> Result<Project, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let n = sqlx::query(
            "UPDATE projects SET \
             name           = COALESCE($1, name), \
             status         = COALESCE($2, status), \
             billing_method = COALESCE($3, billing_method), \
             budget_amount  = COALESCE($4, budget_amount), \
             end_date       = COALESCE($5, end_date), \
             notes          = COALESCE($6, notes), \
             updated_at     = NOW() \
             WHERE id = $7 AND organization_id = $8",
        )
        .bind(input.name)
        .bind(input.status)
        .bind(input.billing_method)
        .bind(input.budget_amount)
        .bind(input.end_date)
        .bind(input.notes)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM projects WHERE id = $1 AND organization_id = $2")
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

    /// Profitability summary for a single project.
    pub async fn project_summary(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<ProjectSummary, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: SummaryRow = sqlx::query_as(
            "SELECT \
               p.id         AS project_id, \
               p.name       AS project_name, \
               COALESCE(inv.total, 0) AS total_invoiced, \
               COALESCE(exp.total, 0) AS total_expenses, \
               COALESCE(te.total,  0) AS total_time_cost, \
               COALESCE(inv.total, 0) \
                 - COALESCE(exp.total, 0) \
                 - COALESCE(te.total, 0) AS net \
             FROM projects p \
             LEFT JOIN ( \
               SELECT te2.project_id, SUM(il.quantity * il.unit_price) AS total \
               FROM time_entries te2 \
               JOIN invoice_lines il ON il.id = te2.invoice_line_id \
               WHERE te2.organization_id = $1 AND te2.invoice_line_id IS NOT NULL \
               GROUP BY te2.project_id \
             ) inv ON inv.project_id = p.id \
             LEFT JOIN ( \
               SELECT project_id, SUM(amount) AS total \
               FROM expenses \
               WHERE organization_id = $1 \
               GROUP BY project_id \
             ) exp ON exp.project_id = p.id \
             LEFT JOIN ( \
               SELECT project_id, SUM(minutes::BIGINT * hourly_rate / 60) AS total \
               FROM time_entries \
               WHERE organization_id = $1 AND is_billable = TRUE \
               GROUP BY project_id \
             ) te ON te.project_id = p.id \
             WHERE p.organization_id = $1 AND p.id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        Ok(ProjectSummary {
            project_id: row.project_id.to_string(),
            project_name: row.project_name,
            total_invoiced: row.total_invoiced,
            total_expenses: row.total_expenses,
            total_time_cost: row.total_time_cost,
            net: row.net,
        })
    }
}
