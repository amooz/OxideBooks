use oxidebooks_core::models::{CreateInventoryLot, InventoryLot, UpdateInventoryLot};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct LotRow {
    id: Uuid,
    organization_id: Uuid,
    item_id: Uuid,
    lot_number: String,
    expiry_date: Option<Date>,
    quantity: i64,
    cost_per_unit: i64,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: LotRow) -> InventoryLot {
    InventoryLot {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        item_id: r.item_id.to_string(),
        lot_number: r.lot_number,
        expiry_date: r.expiry_date,
        quantity: r.quantity,
        cost_per_unit: r.cost_per_unit,
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str =
    "id, organization_id, item_id, lot_number, expiry_date, quantity, cost_per_unit, notes, created_at, updated_at";

pub struct InventoryLotRepo;

impl InventoryLotRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        item_id: &str,
    ) -> Result<Vec<InventoryLot>, DbError> {
        let org = parse_uuid(org_id)?;
        let iid = parse_uuid(item_id)?;
        // verify item belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM inventory_items WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(iid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<LotRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_lots WHERE item_id = $1 ORDER BY lot_number"
        ))
        .bind(iid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<InventoryLot, DbError> {
        let org = parse_uuid(org_id)?;
        let lid = parse_uuid(id)?;
        let row: LotRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_lots WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(lid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInventoryLot,
    ) -> Result<InventoryLot, DbError> {
        let org = parse_uuid(org_id)?;
        let iid = parse_uuid(&input.item_id)?;
        // verify item belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM inventory_items WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(iid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        if input.quantity < 0 {
            return Err(DbError::Conflict("quantity cannot be negative".into()));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO inventory_lots
                (organization_id, item_id, lot_number, expiry_date, quantity, cost_per_unit, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(org)
        .bind(iid)
        .bind(&input.lot_number)
        .bind(input.expiry_date)
        .bind(input.quantity)
        .bind(input.cost_per_unit)
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
        input: UpdateInventoryLot,
    ) -> Result<InventoryLot, DbError> {
        let org = parse_uuid(org_id)?;
        let lid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE inventory_lots SET
             expiry_date   = COALESCE($3, expiry_date),
             quantity      = COALESCE($4, quantity),
             cost_per_unit = COALESCE($5, cost_per_unit),
             notes         = COALESCE($6, notes),
             updated_at    = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(lid)
        .bind(input.expiry_date)
        .bind(input.quantity)
        .bind(input.cost_per_unit)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// List lots expiring within `days` days across the org.
    pub async fn list_expiring(
        pool: &PgPool,
        org_id: &str,
        days: i64,
    ) -> Result<Vec<InventoryLot>, DbError> {
        let org = parse_uuid(org_id)?;
        let cutoff = time::OffsetDateTime::now_utc().date() + time::Duration::days(days);
        let rows: Vec<LotRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_lots
             WHERE organization_id = $1
               AND expiry_date IS NOT NULL
               AND expiry_date <= $2
               AND quantity > 0
             ORDER BY expiry_date ASC"
        ))
        .bind(org)
        .bind(cutoff)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }
}
