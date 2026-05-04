use oxidebooks_core::models::{
    CreateInventoryReorderRequest, InventoryReorderRequest, SubmitInventoryReorderRequest,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

const COLS: &str = "id, organization_id, product_id, supplier_id, requested_qty, status, \
     purchase_order_id, notes, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct ReorderRow {
    id: Uuid,
    organization_id: Uuid,
    product_id: Uuid,
    supplier_id: Option<Uuid>,
    requested_qty: i64,
    status: String,
    purchase_order_id: Option<Uuid>,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ReorderRow) -> InventoryReorderRequest {
    InventoryReorderRequest {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        product_id: r.product_id.to_string(),
        supplier_id: r.supplier_id.map(|u| u.to_string()),
        requested_qty: r.requested_qty,
        status: r.status,
        purchase_order_id: r.purchase_order_id.map(|u| u.to_string()),
        notes: r.notes,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct InventoryReorderRequestRepo;

impl InventoryReorderRequestRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<InventoryReorderRequest>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<ReorderRow> = if let Some(s) = status {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM inventory_reorder_requests \
                 WHERE organization_id = $1 AND status = $2 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .bind(s)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(&format!(
                "SELECT {COLS} FROM inventory_reorder_requests \
                 WHERE organization_id = $1 ORDER BY created_at DESC"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryReorderRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: ReorderRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM inventory_reorder_requests \
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInventoryReorderRequest,
    ) -> Result<InventoryReorderRequest, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let product_uuid = parse_uuid(&input.product_id)?;
        let supplier_uuid = input.supplier_id.as_deref().map(parse_uuid).transpose()?;

        // Fall back to inventory_items.reorder_qty if not specified.
        let requested_qty: i64 = match input.requested_qty {
            Some(q) if q > 0 => q,
            Some(_) => {
                return Err(DbError::Conflict("requested_qty must be > 0".into()));
            }
            None => {
                let fallback: Option<i64> = sqlx::query_scalar(
                    "SELECT reorder_qty FROM inventory_items \
                     WHERE organization_id = $1 AND product_id = $2",
                )
                .bind(org_uuid)
                .bind(product_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?
                .flatten();
                let q = fallback.unwrap_or(0);
                if q <= 0 {
                    return Err(DbError::Conflict(
                        "requested_qty not provided and product has no reorder_qty set".into(),
                    ));
                }
                q
            }
        };

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO inventory_reorder_requests \
             (organization_id, product_id, supplier_id, requested_qty, notes) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(product_uuid)
        .bind(supplier_uuid)
        .bind(requested_qty)
        .bind(&input.notes)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    /// Transitions the request from 'pending' → 'ordered' and creates a draft purchase order.
    pub async fn submit(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: SubmitInventoryReorderRequest,
    ) -> Result<InventoryReorderRequest, DbError> {
        let req = Self::get_by_id(pool, org_id, id).await?;
        if req.status != "pending" {
            return Err(DbError::Conflict(
                "only pending requests can be submitted".into(),
            ));
        }
        let supplier_id = req.supplier_id.as_deref().ok_or_else(|| {
            DbError::Conflict("supplier_id is required to submit a reorder request".into())
        })?;

        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let supplier_uuid = parse_uuid(supplier_id)?;
        let product_uuid = parse_uuid(&req.product_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Generate PO number.
        let next_val: i64 = sqlx::query_scalar(
            "INSERT INTO po_counters (organization_id, next_val) VALUES ($1, 2) \
             ON CONFLICT (organization_id) DO UPDATE \
             SET next_val = po_counters.next_val + 1 RETURNING next_val - 1",
        )
        .bind(org_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
        let po_number = format!("PO-{:05}", next_val);

        let today = time::OffsetDateTime::now_utc().date();
        let expected_date: Option<time::Date> = input
            .delivery_date
            .as_deref()
            .map(|s| {
                let fmt =
                    time::format_description::parse("[year]-[month]-[day]").expect("static format");
                time::Date::parse(s, &fmt)
                    .map_err(|_| DbError::Conflict(format!("invalid delivery_date: {s}")))
            })
            .transpose()?;

        // Get product name for PO line description.
        let product_name: String = sqlx::query_scalar("SELECT name FROM products WHERE id = $1")
            .bind(product_uuid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_err)?
            .unwrap_or_else(|| "Reorder item".to_string());

        let po_id: Uuid = sqlx::query_scalar(
            "INSERT INTO purchase_orders \
             (organization_id, po_number, contact_id, order_date, expected_date, currency) \
             VALUES ($1,$2,$3,$4,$5,'USD') RETURNING id",
        )
        .bind(org_uuid)
        .bind(&po_number)
        .bind(supplier_uuid)
        .bind(today)
        .bind(expected_date)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO purchase_order_lines \
             (po_id, product_id, description, quantity, unit_price, tax_rate, sort_order) \
             VALUES ($1,$2,$3,$4,0,0,0)",
        )
        .bind(po_id)
        .bind(product_uuid)
        .bind(&product_name)
        .bind(req.requested_qty)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE inventory_reorder_requests \
             SET status = 'ordered', purchase_order_id = $1, updated_at = NOW() \
             WHERE id = $2",
        )
        .bind(po_id)
        .bind(id_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn cancel(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<InventoryReorderRequest, DbError> {
        let req = Self::get_by_id(pool, org_id, id).await?;
        if req.status != "pending" {
            return Err(DbError::Conflict(
                "only pending requests can be cancelled".into(),
            ));
        }
        let id_uuid = parse_uuid(id)?;
        sqlx::query(
            "UPDATE inventory_reorder_requests \
             SET status = 'cancelled', updated_at = NOW() WHERE id = $1",
        )
        .bind(id_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    /// Scans all inventory items below their reorder_point and creates a pending reorder request
    /// for each that does not already have one. Returns the newly created requests.
    pub async fn trigger_reorders(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<InventoryReorderRequest>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct CandidateRow {
            product_id: Uuid,
            reorder_qty: i64,
        }

        let candidates: Vec<CandidateRow> = sqlx::query_as(
            "SELECT ii.product_id, ii.reorder_qty
             FROM inventory_items ii
             WHERE ii.organization_id = $1
               AND ii.reorder_qty > 0
               AND ii.quantity_on_hand <= ii.reorder_point
               AND NOT EXISTS (
                   SELECT 1 FROM inventory_reorder_requests r
                   WHERE r.organization_id = $1
                     AND r.product_id = ii.product_id
                     AND r.status = 'pending'
               )",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut created = Vec::new();
        for c in candidates {
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO inventory_reorder_requests \
                 (organization_id, product_id, requested_qty) \
                 VALUES ($1,$2,$3) RETURNING id",
            )
            .bind(org_uuid)
            .bind(c.product_id)
            .bind(c.reorder_qty)
            .fetch_one(pool)
            .await
            .map_err(map_sqlx_err)?;
            let req = Self::get_by_id(pool, org_id, &id.to_string()).await?;
            created.push(req);
        }
        Ok(created)
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
