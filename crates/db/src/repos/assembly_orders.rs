use oxidebooks_core::models::{AssemblyOrder, AssemblyOrderLine, CreateAssemblyOrder};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct OrderRow {
    id: Uuid,
    organization_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    status: String,
    build_date: Option<Date>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn order_from_row(r: OrderRow) -> AssemblyOrder {
    AssemblyOrder {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        product_id: r.product_id.to_string(),
        quantity: r.quantity,
        status: r.status,
        build_date: r.build_date,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    assembly_order_id: Uuid,
    component_id: Uuid,
    quantity_required: i32,
    created_at: OffsetDateTime,
}

fn line_from_row(r: LineRow) -> AssemblyOrderLine {
    AssemblyOrderLine {
        id: r.id.to_string(),
        assembly_order_id: r.assembly_order_id.to_string(),
        component_id: r.component_id.to_string(),
        quantity_required: r.quantity_required,
        created_at: r.created_at,
    }
}

const ORDER_COLS: &str =
    "id, organization_id, product_id, quantity, status, build_date, notes, created_at, updated_at";

pub struct AssemblyOrderRepo;

impl AssemblyOrderRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<AssemblyOrder>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<OrderRow> = sqlx::query_as(&format!(
            "SELECT {ORDER_COLS} FROM assembly_orders \
             WHERE organization_id = $1 ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(order_from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<AssemblyOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: OrderRow = sqlx::query_as(&format!(
            "SELECT {ORDER_COLS} FROM assembly_orders WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(order_from_row(row))
    }

    pub async fn list_lines(
        pool: &PgPool,
        org_id: &str,
        order_id: &str,
    ) -> Result<Vec<AssemblyOrderLine>, DbError> {
        Self::get_by_id(pool, org_id, order_id).await?;
        let order_uuid = parse_uuid(order_id)?;
        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT id, assembly_order_id, component_id, quantity_required, created_at \
             FROM assembly_order_lines WHERE assembly_order_id = $1 ORDER BY created_at ASC",
        )
        .bind(order_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(line_from_row).collect())
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateAssemblyOrder,
    ) -> Result<AssemblyOrder, DbError> {
        if input.quantity <= 0 {
            return Err(DbError::Conflict("quantity must be positive".into()));
        }
        if input.components.is_empty() {
            return Err(DbError::Conflict(
                "assembly order must have at least one component".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let product_uuid = parse_uuid(&input.product_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let order_id: Uuid = sqlx::query_scalar(
            "INSERT INTO assembly_orders \
             (organization_id, product_id, quantity, build_date, notes) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(product_uuid)
        .bind(input.quantity)
        .bind(input.build_date)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.components {
            if line.quantity_required <= 0 {
                return Err(DbError::Conflict(
                    "component quantity_required must be positive".into(),
                ));
            }
            let comp_uuid = parse_uuid(&line.component_id)?;
            sqlx::query(
                "INSERT INTO assembly_order_lines \
                 (assembly_order_id, component_id, quantity_required) \
                 VALUES ($1,$2,$3)",
            )
            .bind(order_id)
            .bind(comp_uuid)
            .bind(line.quantity_required)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &order_id.to_string()).await
    }

    /// Executes the build: deducts component quantities, adds finished-good quantity,
    /// transitions status to 'built'. Fails if any component has insufficient stock.
    pub async fn build(pool: &PgPool, org_id: &str, id: &str) -> Result<AssemblyOrder, DbError> {
        let order = Self::get_by_id(pool, org_id, id).await?;
        if order.status != "pending" {
            return Err(DbError::Conflict(
                "only pending assembly orders can be built".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let order_uuid = parse_uuid(id)?;
        let product_uuid = parse_uuid(&order.product_id)?;

        let lines: Vec<LineRow> = sqlx::query_as(
            "SELECT id, assembly_order_id, component_id, quantity_required, created_at \
             FROM assembly_order_lines WHERE assembly_order_id = $1",
        )
        .bind(order_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Verify sufficient stock for each component.
        for line in &lines {
            let stock: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(quantity_on_hand), 0) FROM inventory_items \
                 WHERE organization_id = $1 AND product_id = $2",
            )
            .bind(org_uuid)
            .bind(line.component_id)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;

            let required = line.quantity_required as i64 * order.quantity as i64;
            if stock < required {
                return Err(DbError::Conflict(format!(
                    "insufficient stock for component {}: have {stock}, need {required}",
                    line.component_id
                )));
            }
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Deduct each component.
        for line in &lines {
            let required = line.quantity_required as i64 * order.quantity as i64;
            sqlx::query(
                "UPDATE inventory_items \
                 SET quantity_on_hand = quantity_on_hand - $1, updated_at = NOW() \
                 WHERE organization_id = $2 AND product_id = $3",
            )
            .bind(required)
            .bind(org_uuid)
            .bind(line.component_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // Add finished goods.
        sqlx::query(
            "UPDATE inventory_items \
             SET quantity_on_hand = quantity_on_hand + $1, updated_at = NOW() \
             WHERE organization_id = $2 AND product_id = $3",
        )
        .bind(order.quantity as i64)
        .bind(org_uuid)
        .bind(product_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE assembly_orders SET status = 'built', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(order_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn cancel(pool: &PgPool, org_id: &str, id: &str) -> Result<AssemblyOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let rows = sqlx::query(
            "UPDATE assembly_orders SET status = 'cancelled', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if rows == 0 {
            return Err(DbError::Conflict(
                "only pending assembly orders can be cancelled".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
