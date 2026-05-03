use oxidebooks_core::models::{CreateGrn, GoodsReceiptNote, GrnLine};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn parse_opt_uuid(s: Option<&str>) -> Result<Option<Uuid>, DbError> {
    match s {
        None => Ok(None),
        Some(v) => parse_uuid(v).map(Some),
    }
}

#[derive(sqlx::FromRow)]
struct GrnRow {
    id: Uuid,
    organization_id: Uuid,
    purchase_order_id: Uuid,
    receipt_date: Date,
    reference: Option<String>,
    notes: Option<String>,
    status: String,
    created_by: String,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    grn_id: Uuid,
    po_line_id: Uuid,
    item_id: Option<Uuid>,
    lot_id: Option<Uuid>,
    quantity_received: i64,
    unit_cost: i64,
}

async fn load_lines(pool: &PgPool, grn_id: Uuid) -> Result<Vec<GrnLine>, DbError> {
    let rows: Vec<LineRow> = sqlx::query_as(
        "SELECT id, grn_id, po_line_id, item_id, lot_id, quantity_received, unit_cost
         FROM grn_lines WHERE grn_id = $1 ORDER BY id",
    )
    .bind(grn_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows
        .into_iter()
        .map(|r| GrnLine {
            id: r.id.to_string(),
            grn_id: r.grn_id.to_string(),
            po_line_id: r.po_line_id.to_string(),
            item_id: r.item_id.map(|u| u.to_string()),
            lot_id: r.lot_id.map(|u| u.to_string()),
            quantity_received: r.quantity_received,
            unit_cost: r.unit_cost,
        })
        .collect())
}

async fn grn_from_row(pool: &PgPool, r: GrnRow) -> Result<GoodsReceiptNote, DbError> {
    let lines = load_lines(pool, r.id).await?;
    Ok(GoodsReceiptNote {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        purchase_order_id: r.purchase_order_id.to_string(),
        receipt_date: r.receipt_date,
        reference: r.reference,
        notes: r.notes,
        status: r.status,
        created_by: r.created_by,
        lines,
        created_at: r.created_at,
    })
}

pub struct GrnRepo;

impl GrnRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        po_id: &str,
    ) -> Result<Vec<GoodsReceiptNote>, DbError> {
        let org = parse_uuid(org_id)?;
        let po = parse_uuid(po_id)?;
        // verify PO belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM purchase_orders WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(po)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let rows: Vec<GrnRow> = sqlx::query_as(
            "SELECT id, organization_id, purchase_order_id, receipt_date, reference, notes,
                    status, created_by, created_at
             FROM goods_receipt_notes
             WHERE organization_id = $1 AND purchase_order_id = $2
             ORDER BY receipt_date DESC, created_at DESC",
        )
        .bind(org)
        .bind(po)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(grn_from_row(pool, row).await?);
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<GoodsReceiptNote, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(id)?;
        let row: GrnRow = sqlx::query_as(
            "SELECT id, organization_id, purchase_order_id, receipt_date, reference, notes,
                    status, created_by, created_at
             FROM goods_receipt_notes WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        grn_from_row(pool, row).await
    }

    /// Create a GRN in draft status (no inventory movements yet).
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateGrn,
    ) -> Result<GoodsReceiptNote, DbError> {
        let org = parse_uuid(org_id)?;
        let po = parse_uuid(&input.purchase_order_id)?;

        // verify PO belongs to org
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM purchase_orders WHERE organization_id = $1 AND id = $2")
                .bind(org)
                .bind(po)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        // validate that every po_line_id belongs to this PO
        for line in &input.lines {
            let pol = parse_uuid(&line.po_line_id)?;
            let ok: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM purchase_order_lines WHERE id = $1 AND po_id = $2")
                    .bind(pol)
                    .bind(po)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            if ok.is_none() {
                return Err(DbError::Conflict(format!(
                    "po_line_id {} does not belong to this purchase order",
                    line.po_line_id
                )));
            }
        }

        let grn_id: Uuid = sqlx::query_scalar(
            "INSERT INTO goods_receipt_notes
                (organization_id, purchase_order_id, receipt_date, reference, notes, created_by)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(org)
        .bind(po)
        .bind(input.receipt_date)
        .bind(&input.reference)
        .bind(&input.notes)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let pol = parse_uuid(&line.po_line_id)?;
            let item = parse_opt_uuid(line.item_id.as_deref())?;
            let lot = parse_opt_uuid(line.lot_id.as_deref())?;
            sqlx::query(
                "INSERT INTO grn_lines (grn_id, po_line_id, item_id, lot_id, quantity_received, unit_cost)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(grn_id)
            .bind(pol)
            .bind(item)
            .bind(lot)
            .bind(line.quantity_received)
            .bind(line.unit_cost)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, org_id, &grn_id.to_string()).await
    }

    /// Post the GRN: update po_line.quantity_received, create inventory movements,
    /// update lot quantities, and flip status to 'posted'.
    pub async fn post(pool: &PgPool, org_id: &str, id: &str) -> Result<GoodsReceiptNote, DbError> {
        let org = parse_uuid(org_id)?;
        let gid = parse_uuid(id)?;

        let row: GrnRow = sqlx::query_as(
            "SELECT id, organization_id, purchase_order_id, receipt_date, reference, notes,
                    status, created_by, created_at
             FROM goods_receipt_notes WHERE organization_id = $1 AND id = $2",
        )
        .bind(org)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        if row.status == "posted" {
            return Err(DbError::Conflict("GRN is already posted".into()));
        }

        let lines = load_lines(pool, gid).await?;
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        for line in &lines {
            let pol = parse_uuid(&line.po_line_id)?;

            // Increment quantity_received on po_line
            sqlx::query(
                "UPDATE purchase_order_lines
                 SET quantity_received = quantity_received + $1
                 WHERE id = $2",
            )
            .bind(line.quantity_received)
            .bind(pol)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

            // Create inventory movement if item_id is set
            if let Some(ref item_id_str) = line.item_id {
                let item = parse_uuid(item_id_str)?;
                sqlx::query(
                    "INSERT INTO inventory_movements
                        (organization_id, item_id, movement_type, quantity, unit_cost, reference_id)
                     VALUES ($1, $2, 'receipt', $3, $4, $5)",
                )
                .bind(org)
                .bind(item)
                .bind(line.quantity_received)
                .bind(line.unit_cost)
                .bind(gid)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;

                // Update inventory_items quantity_on_hand
                sqlx::query(
                    "UPDATE inventory_items
                     SET quantity_on_hand = quantity_on_hand + $1
                     WHERE id = $2",
                )
                .bind(line.quantity_received)
                .bind(item)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;

                // Update lot quantity if lot_id is set
                if let Some(ref lot_id_str) = line.lot_id {
                    let lot = parse_uuid(lot_id_str)?;
                    sqlx::query(
                        "UPDATE inventory_lots
                         SET quantity = quantity + $1, updated_at = now()
                         WHERE id = $2",
                    )
                    .bind(line.quantity_received)
                    .bind(lot)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_sqlx_err)?;
                }
            }
        }

        // Mark GRN as posted
        sqlx::query("UPDATE goods_receipt_notes SET status = 'posted' WHERE id = $1")
            .bind(gid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        // Update PO status: if all lines fully received, set to 'received'
        sqlx::query(
            "UPDATE purchase_orders SET status = 'received', updated_at = now()
             WHERE id = $1
               AND NOT EXISTS (
                   SELECT 1 FROM purchase_order_lines
                   WHERE po_id = $1 AND quantity_received < quantity
               )
               AND status != 'voided'",
        )
        .bind(row.purchase_order_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }
}
