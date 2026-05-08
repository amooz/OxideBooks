use oxidebooks_core::models::{
    CreatePendingTransfer, CreateStockAdjustment, CreateWarehouse, InventoryTransfer,
    StockAdjustment, StockSummaryRow, TransferStock, UpdateWarehouse, Warehouse, WarehouseStock,
    WarehouseStockLine,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct WhRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    code: Option<String>,
    address: Option<String>,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: WhRow) -> Warehouse {
    Warehouse {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        name: r.name,
        code: r.code,
        address: r.address,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct TransferRow {
    id: Uuid,
    organization_id: Uuid,
    from_warehouse_id: Uuid,
    to_warehouse_id: Uuid,
    item_id: Uuid,
    quantity: i64,
    notes: Option<String>,
    status: String,
    transferred_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

fn transfer_from_row(r: TransferRow) -> InventoryTransfer {
    InventoryTransfer {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        from_warehouse_id: r.from_warehouse_id.to_string(),
        to_warehouse_id: r.to_warehouse_id.to_string(),
        item_id: r.item_id.to_string(),
        quantity: r.quantity,
        notes: r.notes,
        status: r.status,
        transferred_at: r.transferred_at,
        created_at: r.created_at,
    }
}

const COLS: &str = "id, organization_id, name, code, address, is_active, created_at, updated_at";
const TRANSFER_COLS: &str = "id, organization_id, from_warehouse_id, to_warehouse_id, item_id, \
     quantity, notes, status, transferred_at, created_at";

pub struct WarehouseRepo;

impl WarehouseRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Warehouse>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<WhRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM warehouses WHERE organization_id = $1 ORDER BY name"
        ))
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Warehouse, DbError> {
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(id)?;
        let row: WhRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM warehouses WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(wid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateWarehouse,
    ) -> Result<Warehouse, DbError> {
        let org = parse_uuid(org_id)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO warehouses (organization_id, name, code, address)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(org)
        .bind(&input.name)
        .bind(&input.code)
        .bind(&input.address)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateWarehouse,
    ) -> Result<Warehouse, DbError> {
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE warehouses SET
             name       = COALESCE($3, name),
             code       = COALESCE($4, code),
             address    = COALESCE($5, address),
             is_active  = COALESCE($6, is_active),
             updated_at = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(wid)
        .bind(&input.name)
        .bind(&input.code)
        .bind(&input.address)
        .bind(input.is_active)
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
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM warehouses WHERE organization_id = $1 AND id = $2")
            .bind(org)
            .bind(wid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Get current stock levels for a warehouse.
    pub async fn stock(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<Vec<WarehouseStock>, DbError> {
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(id)?;
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM warehouses WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(wid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let rows: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT ws.item_id, p.name, ws.quantity
             FROM warehouse_stock ws
             JOIN inventory_items ii ON ii.id = ws.item_id
             JOIN products p ON p.id = ii.product_id
             WHERE ws.warehouse_id = $1
             ORDER BY p.name",
        )
        .bind(wid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|(item_id, product_name, quantity)| WarehouseStock {
                item_id: item_id.to_string(),
                product_name,
                quantity,
            })
            .collect())
    }

    /// Immediate atomic stock transfer (creates as 'completed').
    pub async fn transfer(
        pool: &PgPool,
        org_id: &str,
        input: TransferStock,
    ) -> Result<InventoryTransfer, DbError> {
        let org = parse_uuid(org_id)?;
        let from_wh = parse_uuid(&input.from_warehouse_id)?;
        let to_wh = parse_uuid(&input.to_warehouse_id)?;
        let item_id = parse_uuid(&input.item_id)?;

        if from_wh == to_wh {
            return Err(DbError::Conflict(
                "source and destination warehouses must differ".into(),
            ));
        }
        if input.quantity <= 0 {
            return Err(DbError::Conflict("quantity must be positive".into()));
        }

        for wh in [from_wh, to_wh] {
            let ex: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM warehouses WHERE organization_id = $1 AND id = $2")
                    .bind(org)
                    .bind(wh)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            if ex.is_none() {
                return Err(DbError::NotFound);
            }
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let n = sqlx::query(
            "UPDATE warehouse_stock SET quantity = quantity - $3, updated_at = now()
             WHERE warehouse_id = $1 AND item_id = $2 AND quantity >= $3",
        )
        .bind(from_wh)
        .bind(item_id)
        .bind(input.quantity)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "insufficient stock in source warehouse".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO warehouse_stock (warehouse_id, item_id, quantity)
             VALUES ($1, $2, $3)
             ON CONFLICT (warehouse_id, item_id)
             DO UPDATE SET quantity = warehouse_stock.quantity + EXCLUDED.quantity,
                           updated_at = now()",
        )
        .bind(to_wh)
        .bind(item_id)
        .bind(input.quantity)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let transfer_id: Uuid = sqlx::query_scalar(
            "INSERT INTO inventory_transfers
                (organization_id, from_warehouse_id, to_warehouse_id, item_id, quantity, notes, status)
             VALUES ($1, $2, $3, $4, $5, $6, 'completed')
             RETURNING id",
        )
        .bind(org)
        .bind(from_wh)
        .bind(to_wh)
        .bind(item_id)
        .bind(input.quantity)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_transfer_by_id(pool, transfer_id).await
    }

    /// Create a pending transfer (stock not moved yet).
    pub async fn create_pending_transfer(
        pool: &PgPool,
        org_id: &str,
        input: CreatePendingTransfer,
    ) -> Result<InventoryTransfer, DbError> {
        let org = parse_uuid(org_id)?;
        let from_wh = parse_uuid(&input.from_warehouse_id)?;
        let to_wh = parse_uuid(&input.to_warehouse_id)?;
        let item_id = parse_uuid(&input.item_id)?;

        if from_wh == to_wh {
            return Err(DbError::Conflict(
                "source and destination warehouses must differ".into(),
            ));
        }
        if input.quantity <= 0 {
            return Err(DbError::Conflict("quantity must be positive".into()));
        }

        for wh in [from_wh, to_wh] {
            let ex: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM warehouses WHERE organization_id = $1 AND id = $2")
                    .bind(org)
                    .bind(wh)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            if ex.is_none() {
                return Err(DbError::NotFound);
            }
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO inventory_transfers
                (organization_id, from_warehouse_id, to_warehouse_id, item_id, quantity, notes, status)
             VALUES ($1, $2, $3, $4, $5, $6, 'pending')
             RETURNING id",
        )
        .bind(org)
        .bind(from_wh)
        .bind(to_wh)
        .bind(item_id)
        .bind(input.quantity)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_transfer_by_id(pool, id).await
    }

    /// Receive a pending transfer: move stock and mark completed.
    pub async fn receive_transfer(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryTransfer, DbError> {
        let org = parse_uuid(org_id)?;
        let tid = parse_uuid(id)?;

        let row: Option<TransferRow> = sqlx::query_as(&format!(
            "SELECT {TRANSFER_COLS} FROM inventory_transfers
             WHERE id = $1 AND organization_id = $2 AND status = 'pending'"
        ))
        .bind(tid)
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row = row.ok_or(DbError::NotFound)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Deduct from source.
        let n = sqlx::query(
            "UPDATE warehouse_stock SET quantity = quantity - $3, updated_at = now()
             WHERE warehouse_id = $1 AND item_id = $2 AND quantity >= $3",
        )
        .bind(row.from_warehouse_id)
        .bind(row.item_id)
        .bind(row.quantity)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "insufficient stock in source warehouse".into(),
            ));
        }

        // Add to destination.
        sqlx::query(
            "INSERT INTO warehouse_stock (warehouse_id, item_id, quantity)
             VALUES ($1, $2, $3)
             ON CONFLICT (warehouse_id, item_id)
             DO UPDATE SET quantity = warehouse_stock.quantity + EXCLUDED.quantity,
                           updated_at = now()",
        )
        .bind(row.to_warehouse_id)
        .bind(row.item_id)
        .bind(row.quantity)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Mark completed.
        sqlx::query(
            "UPDATE inventory_transfers SET status = 'completed', transferred_at = now()
             WHERE id = $1",
        )
        .bind(tid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_transfer_by_id(pool, tid).await
    }

    /// Cancel a pending transfer.
    pub async fn cancel_transfer(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryTransfer, DbError> {
        let org = parse_uuid(org_id)?;
        let tid = parse_uuid(id)?;

        let n = sqlx::query(
            "UPDATE inventory_transfers SET status = 'cancelled'
             WHERE id = $1 AND organization_id = $2 AND status = 'pending'",
        )
        .bind(tid)
        .bind(org)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }

        Self::get_transfer_by_id(pool, tid).await
    }

    /// List transfers for the organization (most recent first).
    pub async fn list_transfers(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<InventoryTransfer>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows: Vec<TransferRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {TRANSFER_COLS} FROM inventory_transfers
                 WHERE organization_id = $1 AND status = $2
                 ORDER BY created_at DESC LIMIT $3"
            ))
            .bind(org)
            .bind(s)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {TRANSFER_COLS} FROM inventory_transfers
                 WHERE organization_id = $1
                 ORDER BY created_at DESC LIMIT $2"
            ))
            .bind(org)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(transfer_from_row).collect())
    }

    /// Get a single transfer by ID (scoped to org).
    pub async fn get_transfer(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryTransfer, DbError> {
        let org = parse_uuid(org_id)?;
        let tid = parse_uuid(id)?;
        let row: Option<TransferRow> = sqlx::query_as(&format!(
            "SELECT {TRANSFER_COLS} FROM inventory_transfers
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(tid)
        .bind(org)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
        row.map(transfer_from_row).ok_or(DbError::NotFound)
    }

    async fn get_transfer_by_id(pool: &PgPool, id: Uuid) -> Result<InventoryTransfer, DbError> {
        let row: TransferRow = sqlx::query_as(&format!(
            "SELECT {TRANSFER_COLS} FROM inventory_transfers WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(transfer_from_row(row))
    }

    /// Apply a manual stock adjustment (positive or negative delta).
    pub async fn adjust_stock(
        pool: &PgPool,
        org_id: &str,
        warehouse_id: &str,
        input: CreateStockAdjustment,
    ) -> Result<StockAdjustment, DbError> {
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(warehouse_id)?;
        let item_id = parse_uuid(&input.item_id)?;

        // Verify warehouse belongs to org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM warehouses WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(wid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Upsert stock row; check non-negative result.
        let new_qty: Option<i64> = sqlx::query_scalar(
            "INSERT INTO warehouse_stock (warehouse_id, item_id, quantity)
             VALUES ($1, $2, $3)
             ON CONFLICT (warehouse_id, item_id)
             DO UPDATE SET quantity = warehouse_stock.quantity + EXCLUDED.quantity,
                           updated_at = now()
             RETURNING quantity",
        )
        .bind(wid)
        .bind(item_id)
        .bind(input.quantity_delta)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if new_qty.unwrap_or(0) < 0 {
            tx.rollback().await.map_err(map_sqlx_err)?;
            return Err(DbError::Conflict(
                "adjustment would result in negative stock".into(),
            ));
        }

        let adj_id: Uuid = sqlx::query_scalar(
            "INSERT INTO stock_adjustments (organization_id, warehouse_id, item_id, quantity_delta, reason)
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(org)
        .bind(wid)
        .bind(item_id)
        .bind(input.quantity_delta)
        .bind(&input.reason)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        let row: (Uuid, Uuid, Uuid, Uuid, i64, String, OffsetDateTime) = sqlx::query_as(
            "SELECT id, organization_id, warehouse_id, item_id, quantity_delta, reason, created_at
             FROM stock_adjustments WHERE id = $1",
        )
        .bind(adj_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(StockAdjustment {
            id: row.0.to_string(),
            organization_id: row.1.to_string(),
            warehouse_id: row.2.to_string(),
            item_id: row.3.to_string(),
            quantity_delta: row.4,
            reason: row.5,
            created_at: row.6,
        })
    }

    /// List stock adjustments for a warehouse.
    pub async fn list_adjustments(
        pool: &PgPool,
        org_id: &str,
        warehouse_id: &str,
        limit: i64,
    ) -> Result<Vec<StockAdjustment>, DbError> {
        let org = parse_uuid(org_id)?;
        let wid = parse_uuid(warehouse_id)?;

        let rows: Vec<(Uuid, Uuid, Uuid, Uuid, i64, String, OffsetDateTime)> = sqlx::query_as(
            "SELECT id, organization_id, warehouse_id, item_id, quantity_delta, reason, created_at
             FROM stock_adjustments
             WHERE organization_id = $1 AND warehouse_id = $2
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(org)
        .bind(wid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| StockAdjustment {
                id: r.0.to_string(),
                organization_id: r.1.to_string(),
                warehouse_id: r.2.to_string(),
                item_id: r.3.to_string(),
                quantity_delta: r.4,
                reason: r.5,
                created_at: r.6,
            })
            .collect())
    }

    /// Cross-warehouse stock summary: totals per item with per-warehouse breakdown.
    pub async fn stock_summary(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<StockSummaryRow>, DbError> {
        let org = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            item_id: Uuid,
            product_name: String,
            warehouse_id: Uuid,
            warehouse_name: String,
            quantity: i64,
        }

        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT ws.item_id, p.name AS product_name,
                    w.id AS warehouse_id, w.name AS warehouse_name,
                    ws.quantity
             FROM warehouse_stock ws
             JOIN warehouses w ON w.id = ws.warehouse_id
             JOIN inventory_items ii ON ii.id = ws.item_id
             JOIN products p ON p.id = ii.product_id
             WHERE w.organization_id = $1
             ORDER BY p.name, w.name",
        )
        .bind(org)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Group by item (query is already ordered by product name then warehouse name).
        let mut result: Vec<StockSummaryRow> = vec![];
        for r in rows {
            let item_key = r.item_id.to_string();
            if result.last().map(|s: &StockSummaryRow| &s.item_id) == Some(&item_key) {
                let last = result.last_mut().unwrap();
                last.total_quantity += r.quantity;
                last.by_warehouse.push(WarehouseStockLine {
                    warehouse_id: r.warehouse_id.to_string(),
                    warehouse_name: r.warehouse_name,
                    quantity: r.quantity,
                });
            } else {
                result.push(StockSummaryRow {
                    item_id: item_key,
                    product_name: r.product_name,
                    total_quantity: r.quantity,
                    by_warehouse: vec![WarehouseStockLine {
                        warehouse_id: r.warehouse_id.to_string(),
                        warehouse_name: r.warehouse_name,
                        quantity: r.quantity,
                    }],
                });
            }
        }

        Ok(result)
    }
}
