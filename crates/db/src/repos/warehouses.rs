use oxidebooks_core::models::{
    CreateWarehouse, InventoryTransfer, TransferStock, UpdateWarehouse, Warehouse, WarehouseStock,
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

const COLS: &str = "id, organization_id, name, code, address, is_active, created_at, updated_at";

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
        // Verify warehouse belongs to org
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

    /// Transfer stock from one warehouse to another.
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

        // Verify both warehouses belong to org
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

        // Deduct from source (must have enough stock)
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

        // Add to destination (upsert)
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
                (organization_id, from_warehouse_id, to_warehouse_id, item_id, quantity, notes)
             VALUES ($1, $2, $3, $4, $5, $6)
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

        let row: (
            Uuid,
            Uuid,
            Uuid,
            Uuid,
            Uuid,
            i64,
            Option<String>,
            OffsetDateTime,
            OffsetDateTime,
        ) = sqlx::query_as(
            "SELECT id, organization_id, from_warehouse_id, to_warehouse_id,
                        item_id, quantity, notes, transferred_at, created_at
                 FROM inventory_transfers WHERE id = $1",
        )
        .bind(transfer_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(InventoryTransfer {
            id: row.0.to_string(),
            organization_id: row.1.to_string(),
            from_warehouse_id: row.2.to_string(),
            to_warehouse_id: row.3.to_string(),
            item_id: row.4.to_string(),
            quantity: row.5,
            notes: row.6,
            transferred_at: row.7,
            created_at: row.8,
        })
    }
}
