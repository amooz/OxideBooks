use oxidebooks_core::models::{
    CreateInventorySerialNumber, InventorySerialNumber, UpdateInventorySerialNumber,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct SerialNumberRow {
    id: Uuid,
    organization_id: Uuid,
    product_id: Uuid,
    serial_number: String,
    status: String,
    lot_id: Option<Uuid>,
    warehouse_id: Option<Uuid>,
    purchase_date: Option<Date>,
    sold_date: Option<Date>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<SerialNumberRow> for InventorySerialNumber {
    fn from(r: SerialNumberRow) -> Self {
        InventorySerialNumber {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            product_id: r.product_id.to_string(),
            serial_number: r.serial_number,
            status: r.status,
            lot_id: r.lot_id.map(|u| u.to_string()),
            warehouse_id: r.warehouse_id.map(|u| u.to_string()),
            purchase_date: r.purchase_date,
            sold_date: r.sold_date,
            notes: r.notes,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLS: &str = "id, organization_id, product_id, serial_number, status, lot_id, \
    warehouse_id, purchase_date, sold_date, notes, created_at, updated_at";

pub struct InventorySerialNumberRepo;

impl InventorySerialNumberRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        product_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<InventorySerialNumber>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let mut query = format!(
            "SELECT {COLS} FROM inventory_serial_numbers \
             WHERE organization_id = $1"
        );
        let mut param_idx = 2usize;

        let mut product_uuid: Option<Uuid> = None;
        if let Some(pid) = product_id {
            product_uuid = Some(parse_uuid(pid)?);
            query.push_str(&format!(" AND product_id = ${param_idx}"));
            param_idx += 1;
        }

        let mut status_owned: Option<String> = None;
        if let Some(s) = status {
            status_owned = Some(s.to_string());
            query.push_str(&format!(" AND status = ${param_idx}"));
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut q = sqlx::query_as::<_, SerialNumberRow>(&query).bind(org_uuid);
        if let Some(pid) = product_uuid {
            q = q.bind(pid);
        }
        if let Some(s) = status_owned {
            q = q.bind(s);
        }

        let rows = q.fetch_all(pool).await.map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(InventorySerialNumber::from).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventorySerialNumber, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: Option<SerialNumberRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_serial_numbers \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.map(InventorySerialNumber::from)
            .ok_or(DbError::NotFound)
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInventorySerialNumber,
    ) -> Result<InventorySerialNumber, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let product_uuid = parse_uuid(&input.product_id)?;
        let lot_uuid = input.lot_id.as_deref().map(parse_uuid).transpose()?;
        let wh_uuid = input.warehouse_id.as_deref().map(parse_uuid).transpose()?;

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO inventory_serial_numbers \
             (id, organization_id, product_id, serial_number, lot_id, warehouse_id, \
              purchase_date, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(product_uuid)
        .bind(&input.serial_number)
        .bind(lot_uuid)
        .bind(wh_uuid)
        .bind(input.purchase_date)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        let row: SerialNumberRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_serial_numbers WHERE id = $1"
        ))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateInventorySerialNumber,
    ) -> Result<InventorySerialNumber, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let lot_uuid = input.lot_id.as_deref().map(parse_uuid).transpose()?;
        let wh_uuid = input.warehouse_id.as_deref().map(parse_uuid).transpose()?;

        let rows = sqlx::query(
            "UPDATE inventory_serial_numbers SET \
             status       = COALESCE($3, status), \
             lot_id       = COALESCE($4, lot_id), \
             warehouse_id = COALESCE($5, warehouse_id), \
             sold_date    = COALESCE($6, sold_date), \
             notes        = COALESCE($7, notes), \
             updated_at   = NOW() \
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .bind(&input.status)
        .bind(lot_uuid)
        .bind(wh_uuid)
        .bind(input.sold_date)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            return Err(DbError::NotFound);
        }

        let row: SerialNumberRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_serial_numbers WHERE id = $1"
        ))
        .bind(id_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "DELETE FROM inventory_serial_numbers WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            Err(DbError::NotFound)
        } else {
            Ok(())
        }
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
