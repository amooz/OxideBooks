use oxidebooks_core::models::{
    CreateInventoryItem, InventoryAdjustment, InventoryItem, InventoryMovement, LowStockItem,
    UpdateInventoryItem,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    organization_id: Uuid,
    product_id: Uuid,
    quantity_on_hand: i64,
    reorder_point: i64,
    cost_per_unit: i64,
    valuation_method: String,
}

#[derive(sqlx::FromRow)]
struct MovementRow {
    id: Uuid,
    organization_id: Uuid,
    item_id: Uuid,
    movement_type: String,
    quantity: i64,
    unit_cost: i64,
    reference_id: Option<Uuid>,
    reference_type: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LowStockRow {
    product_id: Uuid,
    product_name: String,
    quantity_on_hand: i64,
    reorder_point: i64,
}

fn item_from_row(r: ItemRow) -> InventoryItem {
    InventoryItem {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        product_id: r.product_id.to_string(),
        quantity_on_hand: r.quantity_on_hand,
        reorder_point: r.reorder_point,
        cost_per_unit: r.cost_per_unit,
        valuation_method: r.valuation_method,
    }
}

fn movement_from_row(r: MovementRow) -> InventoryMovement {
    InventoryMovement {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        item_id: r.item_id.to_string(),
        movement_type: r.movement_type,
        quantity: r.quantity,
        unit_cost: r.unit_cost,
        reference_id: r.reference_id.map(|u| u.to_string()),
        reference_type: r.reference_type,
        notes: r.notes,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct InventoryRepo;

impl InventoryRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<InventoryItem>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ItemRow> = sqlx::query_as(
            "SELECT id, organization_id, product_id, quantity_on_hand, reorder_point, \
             cost_per_unit, valuation_method \
             FROM inventory_items WHERE organization_id = $1",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(item_from_row).collect())
    }

    pub async fn get_by_product(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
    ) -> Result<InventoryItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let row: ItemRow = sqlx::query_as(
            "SELECT id, organization_id, product_id, quantity_on_hand, reorder_point, \
             cost_per_unit, valuation_method \
             FROM inventory_items WHERE organization_id = $1 AND product_id = $2",
        )
        .bind(org_uuid)
        .bind(prod_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(item_from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInventoryItem,
    ) -> Result<InventoryItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(&input.product_id)?;
        sqlx::query(
            "INSERT INTO inventory_items \
             (organization_id, product_id, quantity_on_hand, reorder_point, cost_per_unit, valuation_method) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (organization_id, product_id) DO UPDATE SET \
             reorder_point = EXCLUDED.reorder_point, \
             cost_per_unit = EXCLUDED.cost_per_unit, \
             valuation_method = EXCLUDED.valuation_method",
        )
        .bind(org_uuid)
        .bind(prod_uuid)
        .bind(input.quantity_on_hand)
        .bind(input.reorder_point)
        .bind(input.cost_per_unit)
        .bind(&input.valuation_method)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if input.quantity_on_hand != 0 {
            // Record initial stock as an adjustment movement
            let item = Self::get_by_product(pool, org_id, &input.product_id).await?;
            sqlx::query(
                "INSERT INTO inventory_movements (organization_id, item_id, movement_type, quantity, unit_cost, notes) \
                 VALUES ($1,$2,'adjustment',$3,$4,'initial stock')",
            )
            .bind(org_uuid)
            .bind(parse_uuid(&item.id)?)
            .bind(input.quantity_on_hand)
            .bind(input.cost_per_unit)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_product(pool, org_id, &input.product_id).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        input: UpdateInventoryItem,
    ) -> Result<InventoryItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let n = sqlx::query(
            "UPDATE inventory_items SET \
             reorder_point    = COALESCE($1, reorder_point), \
             cost_per_unit    = COALESCE($2, cost_per_unit), \
             valuation_method = COALESCE($3, valuation_method) \
             WHERE organization_id = $4 AND product_id = $5",
        )
        .bind(input.reorder_point)
        .bind(input.cost_per_unit)
        .bind(input.valuation_method)
        .bind(org_uuid)
        .bind(prod_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_product(pool, org_id, product_id).await
    }

    pub async fn adjust(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
        adj: InventoryAdjustment,
    ) -> Result<InventoryItem, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let item_id: Uuid = sqlx::query_scalar(
            "UPDATE inventory_items SET \
             quantity_on_hand = quantity_on_hand + $1 \
             WHERE organization_id = $2 AND product_id = $3 \
             RETURNING id",
        )
        .bind(adj.quantity)
        .bind(org_uuid)
        .bind(prod_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let unit_cost = adj.unit_cost.unwrap_or(0);
        sqlx::query(
            "INSERT INTO inventory_movements \
             (organization_id, item_id, movement_type, quantity, unit_cost, notes) \
             VALUES ($1,$2,'adjustment',$3,$4,$5)",
        )
        .bind(org_uuid)
        .bind(item_id)
        .bind(adj.quantity)
        .bind(unit_cost)
        .bind(&adj.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_product(pool, org_id, product_id).await
    }

    pub async fn movements(
        pool: &PgPool,
        org_id: &str,
        product_id: &str,
    ) -> Result<Vec<InventoryMovement>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let prod_uuid = parse_uuid(product_id)?;
        let rows: Vec<MovementRow> = sqlx::query_as(
            "SELECT m.id, m.organization_id, m.item_id, m.movement_type, m.quantity, \
             m.unit_cost, m.reference_id, m.reference_type, m.notes, m.created_at \
             FROM inventory_movements m \
             JOIN inventory_items i ON i.id = m.item_id \
             WHERE m.organization_id = $1 AND i.product_id = $2 \
             ORDER BY m.created_at DESC",
        )
        .bind(org_uuid)
        .bind(prod_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(movement_from_row).collect())
    }

    pub async fn low_stock(pool: &PgPool, org_id: &str) -> Result<Vec<LowStockItem>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<LowStockRow> = sqlx::query_as(
            "SELECT i.product_id, p.name AS product_name, \
             i.quantity_on_hand, i.reorder_point \
             FROM inventory_items i \
             JOIN products p ON p.id = i.product_id \
             WHERE i.organization_id = $1 \
               AND i.quantity_on_hand <= i.reorder_point \
             ORDER BY (i.quantity_on_hand - i.reorder_point) ASC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| LowStockItem {
                product_id: r.product_id.to_string(),
                product_name: r.product_name,
                quantity_on_hand: r.quantity_on_hand,
                reorder_point: r.reorder_point,
            })
            .collect())
    }
}
