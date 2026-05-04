use oxidebooks_core::models::{
    CreateInventoryStocktake, InventoryStocktake, InventoryStocktakeLine, UpdateStocktakeLine,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct StocktakeRow {
    id: Uuid,
    organization_id: Uuid,
    stocktake_date: Date,
    warehouse_id: Option<Uuid>,
    status: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct StocktakeLineRow {
    id: Uuid,
    stocktake_id: Uuid,
    product_id: Uuid,
    system_qty: i64,
    counted_qty: i64,
    variance: i64,
    notes: Option<String>,
}

pub struct InventoryStocktakeRepo;

impl InventoryStocktakeRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<InventoryStocktake>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<StocktakeRow> = if let Some(s) = status {
            sqlx::query_as(
                "SELECT id, organization_id, stocktake_date, warehouse_id, status, notes, \
                 created_at, updated_at \
                 FROM inventory_stocktakes \
                 WHERE organization_id = $1 AND status = $2 \
                 ORDER BY stocktake_date DESC, created_at DESC",
            )
            .bind(org_uuid)
            .bind(s)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, stocktake_date, warehouse_id, status, notes, \
                 created_at, updated_at \
                 FROM inventory_stocktakes \
                 WHERE organization_id = $1 \
                 ORDER BY stocktake_date DESC, created_at DESC",
            )
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let mut result = Vec::with_capacity(rows.len());
        for r in rows {
            result.push(Self::assemble(pool, r).await?);
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryStocktake, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: StocktakeRow = sqlx::query_as(
            "SELECT id, organization_id, stocktake_date, warehouse_id, status, notes, \
             created_at, updated_at \
             FROM inventory_stocktakes \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Self::assemble(pool, row).await
    }

    /// Create a stocktake and snapshot current system quantities for all active inventory items
    /// (optionally filtered to a specific list of product_ids).
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInventoryStocktake,
    ) -> Result<InventoryStocktake, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let wh_uuid = input.warehouse_id.as_deref().map(parse_uuid).transpose()?;

        let id = Uuid::new_v4();
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO inventory_stocktakes \
             (id, organization_id, stocktake_date, warehouse_id, notes) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(input.stocktake_date)
        .bind(wh_uuid)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Snapshot system quantities from inventory_items.
        // If product_ids is provided, restrict to that set; otherwise include all active items.
        if input.product_ids.is_empty() {
            sqlx::query(
                "INSERT INTO inventory_stocktake_lines \
                 (id, stocktake_id, product_id, system_qty) \
                 SELECT gen_random_uuid(), $1, ii.product_id, ii.quantity_on_hand \
                 FROM inventory_items ii \
                 JOIN products p ON p.id = ii.product_id \
                 WHERE ii.organization_id = $2 \
                   AND p.is_active = TRUE",
            )
            .bind(id)
            .bind(org_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        } else {
            let product_uuids: Result<Vec<Uuid>, _> =
                input.product_ids.iter().map(|s| parse_uuid(s)).collect();
            let product_uuids = product_uuids?;
            sqlx::query(
                "INSERT INTO inventory_stocktake_lines \
                 (id, stocktake_id, product_id, system_qty) \
                 SELECT gen_random_uuid(), $1, ii.product_id, ii.quantity_on_hand \
                 FROM inventory_items ii \
                 WHERE ii.organization_id = $2 \
                   AND ii.product_id = ANY($3)",
            )
            .bind(id)
            .bind(org_uuid)
            .bind(&product_uuids)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Update the counted quantity (and optional notes) on a single line.
    pub async fn update_line(
        pool: &PgPool,
        org_id: &str,
        stocktake_id: &str,
        line_id: &str,
        input: UpdateStocktakeLine,
    ) -> Result<InventoryStocktake, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let stocktake_uuid = parse_uuid(stocktake_id)?;
        let line_uuid = parse_uuid(line_id)?;

        let rows = sqlx::query(
            "UPDATE inventory_stocktake_lines isl \
             SET counted_qty = $1, notes = COALESCE($2, isl.notes) \
             FROM inventory_stocktakes ist \
             WHERE isl.id = $3 \
               AND isl.stocktake_id = $4 \
               AND ist.id = $4 \
               AND ist.organization_id = $5 \
               AND ist.status = 'draft'",
        )
        .bind(input.counted_qty)
        .bind(&input.notes)
        .bind(line_uuid)
        .bind(stocktake_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "line not found or stocktake is not in draft state".into(),
            ));
        }

        Self::get_by_id(pool, org_id, stocktake_id).await
    }

    /// Transition status from draft → submitted.
    pub async fn submit(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryStocktake, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE inventory_stocktakes SET status = 'submitted', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::Conflict(
                "stocktake not found or not in draft state".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Transition status from submitted → posted, and apply inventory adjustments for variances.
    pub async fn post(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryStocktake, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let rows = sqlx::query(
            "UPDATE inventory_stocktakes SET status = 'posted', updated_at = NOW() \
             WHERE organization_id = $1 AND id = $2 AND status = 'submitted'",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            tx.rollback().await.map_err(map_sqlx_err)?;
            return Err(DbError::Conflict(
                "stocktake not found or not in submitted state".into(),
            ));
        }

        // Fetch lines with non-zero variance and apply adjustments.
        #[derive(sqlx::FromRow)]
        struct VarianceLine {
            product_id: Uuid,
            variance: i64,
        }

        let variances: Vec<VarianceLine> = sqlx::query_as(
            "SELECT product_id, variance \
             FROM inventory_stocktake_lines \
             WHERE stocktake_id = $1 AND variance <> 0",
        )
        .bind(id_uuid)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for v in variances {
            // Update inventory_items quantity
            sqlx::query(
                "UPDATE inventory_items \
                 SET quantity_on_hand = quantity_on_hand + $1 \
                 WHERE organization_id = $2 AND product_id = $3",
            )
            .bind(v.variance)
            .bind(org_uuid)
            .bind(v.product_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

            // Record movement
            let item_id: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM inventory_items \
                 WHERE organization_id = $1 AND product_id = $2",
            )
            .bind(org_uuid)
            .bind(v.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

            if let Some((item_id,)) = item_id {
                sqlx::query(
                    "INSERT INTO inventory_movements \
                     (organization_id, item_id, movement_type, quantity, unit_cost, notes) \
                     VALUES ($1, $2, 'adjustment', $3, 0, 'stocktake adjustment')",
                )
                .bind(org_uuid)
                .bind(item_id)
                .bind(v.variance)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
            }
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    async fn assemble(pool: &PgPool, r: StocktakeRow) -> Result<InventoryStocktake, DbError> {
        let line_rows: Vec<StocktakeLineRow> = sqlx::query_as(
            "SELECT id, stocktake_id, product_id, system_qty, counted_qty, variance, notes \
             FROM inventory_stocktake_lines \
             WHERE stocktake_id = $1 \
             ORDER BY product_id ASC",
        )
        .bind(r.id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let lines = line_rows
            .into_iter()
            .map(|l| InventoryStocktakeLine {
                id: l.id.to_string(),
                stocktake_id: l.stocktake_id.to_string(),
                product_id: l.product_id.to_string(),
                system_qty: l.system_qty,
                counted_qty: l.counted_qty,
                variance: l.variance,
                notes: l.notes,
            })
            .collect();

        Ok(InventoryStocktake {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            stocktake_date: r.stocktake_date,
            warehouse_id: r.warehouse_id.map(|u| u.to_string()),
            status: r.status,
            notes: r.notes,
            lines,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
