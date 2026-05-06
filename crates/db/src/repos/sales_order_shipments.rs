use oxidebooks_core::models::{CreateSalesOrderShipment, SalesOrderShipment, ShipmentLine};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct ShipmentRow {
    id: Uuid,
    organization_id: Uuid,
    sales_order_id: Uuid,
    shipped_at: Date,
    tracking_number: Option<String>,
    carrier: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    shipment_id: Uuid,
    so_line_id: Uuid,
    product_id: Option<Uuid>,
    quantity_shipped: i64,
}

async fn fetch_lines(pool: &PgPool, shipment_id: Uuid) -> Result<Vec<ShipmentLine>, DbError> {
    let rows: Vec<LineRow> = sqlx::query_as(
        "SELECT id, shipment_id, so_line_id, product_id, quantity_shipped \
         FROM sales_order_shipment_lines WHERE shipment_id = $1 ORDER BY id",
    )
    .bind(shipment_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| ShipmentLine {
            id: r.id.to_string(),
            shipment_id: r.shipment_id.to_string(),
            so_line_id: r.so_line_id.to_string(),
            product_id: r.product_id.map(|u| u.to_string()),
            quantity_shipped: r.quantity_shipped,
        })
        .collect())
}

async fn shipment_from_row(pool: &PgPool, r: ShipmentRow) -> Result<SalesOrderShipment, DbError> {
    let lines = fetch_lines(pool, r.id).await?;
    Ok(SalesOrderShipment {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        sales_order_id: r.sales_order_id.to_string(),
        shipped_at: r.shipped_at,
        tracking_number: r.tracking_number,
        carrier: r.carrier,
        notes: r.notes,
        lines,
        created_at: r.created_at,
    })
}

const COLS: &str =
    "id, organization_id, sales_order_id, shipped_at, tracking_number, carrier, notes, created_at";

pub struct SalesOrderShipmentRepo;

impl SalesOrderShipmentRepo {
    pub async fn list_for_order(
        pool: &PgPool,
        org_id: &str,
        so_id: &str,
    ) -> Result<Vec<SalesOrderShipment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let so_uuid = parse_uuid(so_id)?;
        let rows: Vec<ShipmentRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_order_shipments \
             WHERE organization_id = $1 AND sales_order_id = $2 \
             ORDER BY shipped_at DESC, created_at DESC"
        ))
        .bind(org_uuid)
        .bind(so_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(shipment_from_row(pool, row).await?);
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<SalesOrderShipment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ShipmentRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM sales_order_shipments WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        shipment_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        so_id: &str,
        input: CreateSalesOrderShipment,
    ) -> Result<SalesOrderShipment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let so_uuid = parse_uuid(so_id)?;
        let shipped_at = input
            .shipped_at
            .unwrap_or_else(|| time::OffsetDateTime::now_utc().date());

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        let shipment_id: Uuid = sqlx::query_scalar(
            "INSERT INTO sales_order_shipments \
             (organization_id, sales_order_id, shipped_at, tracking_number, carrier, notes) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(so_uuid)
        .bind(shipped_at)
        .bind(&input.tracking_number)
        .bind(&input.carrier)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let so_line_uuid = parse_uuid(&line.so_line_id)?;
            let product_id: Option<Uuid> =
                sqlx::query_scalar("SELECT product_id FROM sales_order_lines WHERE id = $1")
                    .bind(so_line_uuid)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_sqlx_err)?
                    .flatten();

            sqlx::query(
                "INSERT INTO sales_order_shipment_lines \
                 (shipment_id, so_line_id, product_id, quantity_shipped) \
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(shipment_id)
            .bind(so_line_uuid)
            .bind(product_id)
            .bind(line.quantity_shipped)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &shipment_id.to_string()).await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
