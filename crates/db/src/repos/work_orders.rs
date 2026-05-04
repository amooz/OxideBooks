use oxidebooks_core::models::{CreateWorkOrder, UpdateWorkOrder, WorkOrder, WorkOrderLine};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct WorkOrderRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    scheduled_date: Option<Date>,
    completed_date: Option<Date>,
    invoice_id: Option<Uuid>,
    doc_number: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    work_order_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i32,
    unit_price: i64,
    completed: bool,
    created_at: OffsetDateTime,
}

const WO_COLS: &str = "id, organization_id, contact_id, assigned_to, title, description, \
    status, priority, scheduled_date, completed_date, invoice_id, doc_number, notes, \
    created_at, updated_at";

pub struct WorkOrderRepo;

impl WorkOrderRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<WorkOrder>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<WorkOrderRow> = if let Some(status) = status_filter {
            sqlx::query_as(&format!(
                "SELECT {WO_COLS} FROM work_orders \
                 WHERE organization_id = $1 AND status = $2 \
                 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .bind(status)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {WO_COLS} FROM work_orders \
                 WHERE organization_id = $1 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let mut result = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            result.push(wo_from_row(r, lines));
        }
        Ok(result)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<WorkOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: WorkOrderRow = sqlx::query_as(&format!(
            "SELECT {WO_COLS} FROM work_orders WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        let lines = Self::fetch_lines(pool, row.id).await?;
        Ok(wo_from_row(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateWorkOrder,
    ) -> Result<WorkOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;
        let assigned_uuid = input.assigned_to.as_deref().map(parse_uuid).transpose()?;

        validate_priority(&input.priority)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let wo_id: Uuid = sqlx::query_scalar(
            "INSERT INTO work_orders \
             (organization_id, contact_id, assigned_to, title, description, \
              priority, scheduled_date, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(assigned_uuid)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.priority)
        .bind(input.scheduled_date)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let product_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO work_order_lines \
                 (work_order_id, product_id, description, quantity, unit_price) \
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(wo_id)
            .bind(product_uuid)
            .bind(line.description.as_deref().unwrap_or(""))
            .bind(line.quantity.unwrap_or(1))
            .bind(line.unit_price.unwrap_or(0))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &wo_id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateWorkOrder,
    ) -> Result<WorkOrder, DbError> {
        if let Some(ref p) = input.priority {
            validate_priority(p)?;
        }
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;
        let assigned_uuid = input.assigned_to.as_deref().map(parse_uuid).transpose()?;

        let rows = sqlx::query(
            "UPDATE work_orders SET \
             title          = COALESCE($3, title), \
             contact_id     = COALESCE($4, contact_id), \
             assigned_to    = COALESCE($5, assigned_to), \
             description    = COALESCE($6, description), \
             priority       = COALESCE($7, priority), \
             scheduled_date = COALESCE($8, scheduled_date), \
             completed_date = COALESCE($9, completed_date), \
             notes          = COALESCE($10, notes), \
             updated_at     = NOW() \
             WHERE organization_id = $1 AND id = $2 \
             AND status NOT IN ('invoiced','cancelled')",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.title)
        .bind(contact_uuid)
        .bind(assigned_uuid)
        .bind(&input.description)
        .bind(&input.priority)
        .bind(input.scheduled_date)
        .bind(input.completed_date)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "work order not found or cannot be updated in its current status".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn set_status(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        new_status: &str,
        allowed_from: &[&str],
    ) -> Result<WorkOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let wo = Self::get_by_id(pool, org_id, id).await?;
        if !allowed_from.contains(&wo.status.as_str()) {
            return Err(DbError::Conflict(format!(
                "cannot transition from '{}' to '{new_status}'",
                wo.status
            )));
        }

        let completed_date = if new_status == "completed" {
            Some(time::OffsetDateTime::now_utc().date())
        } else {
            None
        };

        sqlx::query(
            "UPDATE work_orders SET status = $3, \
             completed_date = COALESCE($4, completed_date), \
             updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(new_status)
        .bind(completed_date)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "DELETE FROM work_orders WHERE organization_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "work order not found or can only delete open work orders".into(),
            ));
        }
        Ok(())
    }

    async fn fetch_lines(pool: &PgPool, wo_id: Uuid) -> Result<Vec<WorkOrderLine>, DbError> {
        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT id, work_order_id, product_id, description, quantity, unit_price, \
             completed, created_at \
             FROM work_order_lines WHERE work_order_id = $1 ORDER BY created_at ASC",
        )
        .bind(wo_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(line_from_row).collect())
    }
}

fn wo_from_row(r: WorkOrderRow, lines: Vec<WorkOrderLine>) -> WorkOrder {
    WorkOrder {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.map(|u| u.to_string()),
        assigned_to: r.assigned_to.map(|u| u.to_string()),
        title: r.title,
        description: r.description,
        status: r.status,
        priority: r.priority,
        scheduled_date: r.scheduled_date,
        completed_date: r.completed_date,
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        doc_number: r.doc_number,
        notes: r.notes,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn line_from_row(r: LineRow) -> WorkOrderLine {
    WorkOrderLine {
        id: r.id.to_string(),
        work_order_id: r.work_order_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        description: r.description,
        quantity: r.quantity,
        unit_price: r.unit_price,
        completed: r.completed,
        created_at: r.created_at,
    }
}

fn validate_priority(p: &str) -> Result<(), DbError> {
    match p {
        "low" | "normal" | "high" | "urgent" => Ok(()),
        other => Err(DbError::Conflict(format!(
            "invalid priority '{other}'; must be low, normal, high, or urgent"
        ))),
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
