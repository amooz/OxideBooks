use oxidebooks_core::models::{
    CreatePoLine, CreatePurchaseOrder, PoStatus, PurchaseOrder, PurchaseOrderLine, ReceivePoLine,
    UpdatePurchaseOrder,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PoRow {
    id: Uuid,
    organization_id: Uuid,
    po_number: String,
    contact_id: Uuid,
    status: String,
    order_date: Date,
    expected_date: Option<Date>,
    currency: String,
    notes: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct PoLineRow {
    id: Uuid,
    po_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i64,
    unit_price: i64,
    tax_rate: i64,
    quantity_received: i64,
    sort_order: i32,
}

fn line_from_row(r: PoLineRow) -> PurchaseOrderLine {
    PurchaseOrderLine {
        id: r.id.to_string(),
        po_id: r.po_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        description: r.description,
        quantity: r.quantity,
        unit_price: r.unit_price,
        tax_rate: r.tax_rate,
        quantity_received: r.quantity_received,
        sort_order: r.sort_order,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

async fn fetch_lines(pool: &PgPool, po_id: Uuid) -> Result<Vec<PurchaseOrderLine>, DbError> {
    let rows: Vec<PoLineRow> = sqlx::query_as(
        "SELECT id, po_id, product_id, description, quantity, unit_price, tax_rate, \
         quantity_received, sort_order \
         FROM purchase_order_lines WHERE po_id = $1 ORDER BY sort_order",
    )
    .bind(po_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(line_from_row).collect())
}

async fn po_from_row(pool: &PgPool, r: PoRow) -> Result<PurchaseOrder, DbError> {
    let lines = fetch_lines(pool, r.id).await?;
    Ok(PurchaseOrder {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        po_number: r.po_number,
        contact_id: r.contact_id.to_string(),
        status: PoStatus::from_str(&r.status).unwrap_or(PoStatus::Draft),
        order_date: r.order_date,
        expected_date: r.expected_date,
        currency: r.currency,
        notes: r.notes,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

pub struct PurchaseOrderRepo;

impl PurchaseOrderRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<PurchaseOrder>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<PoRow> = if let Some(s) = status {
            sqlx::query_as(
                "SELECT id, organization_id, po_number, contact_id, status, order_date, \
                 expected_date, currency, notes, created_at, updated_at \
                 FROM purchase_orders WHERE organization_id = $1 AND status = $2 \
                 ORDER BY created_at DESC",
            )
            .bind(org_uuid)
            .bind(s)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, po_number, contact_id, status, order_date, \
                 expected_date, currency, notes, created_at, updated_at \
                 FROM purchase_orders WHERE organization_id = $1 \
                 ORDER BY created_at DESC",
            )
            .bind(org_uuid)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let mut result = Vec::with_capacity(rows.len());
        for r in rows {
            result.push(po_from_row(pool, r).await?);
        }
        Ok(result)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: PoRow = sqlx::query_as(
            "SELECT id, organization_id, po_number, contact_id, status, order_date, \
             expected_date, currency, notes, created_at, updated_at \
             FROM purchase_orders WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        po_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePurchaseOrder,
    ) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(&input.contact_id)?;

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Generate PO number using counter
        let next_val: i64 = sqlx::query_scalar(
            "INSERT INTO po_counters (organization_id, next_val) VALUES ($1, 2) \
             ON CONFLICT (organization_id) DO UPDATE SET next_val = po_counters.next_val + 1 \
             RETURNING next_val - 1",
        )
        .bind(org_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let po_number = format!("PO-{:05}", next_val);

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO purchase_orders \
             (organization_id, po_number, contact_id, order_date, expected_date, currency, notes) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
        )
        .bind(org_uuid)
        .bind(&po_number)
        .bind(contact_uuid)
        .bind(input.order_date)
        .bind(input.expected_date)
        .bind(&input.currency)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, line) in input.lines.into_iter().enumerate() {
            let prod_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO purchase_order_lines \
                 (po_id, product_id, description, quantity, unit_price, tax_rate, sort_order) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(id)
            .bind(prod_uuid)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(i as i32)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdatePurchaseOrder,
    ) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let status_str = input.status.as_ref().map(|s| s.to_string());

        let n = sqlx::query(
            "UPDATE purchase_orders SET \
             status        = COALESCE($1, status), \
             expected_date = COALESCE($2, expected_date), \
             notes         = COALESCE($3, notes), \
             updated_at    = NOW() \
             WHERE id = $4 AND organization_id = $5",
        )
        .bind(status_str)
        .bind(input.expected_date)
        .bind(input.notes)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if n == 0 {
            return Err(DbError::NotFound);
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Record received quantities for PO lines.
    pub async fn receive(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        lines: Vec<ReceivePoLine>,
    ) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let po = Self::get_by_id(pool, org_id, id).await?;
        if matches!(po.status, PoStatus::Voided | PoStatus::Billed) {
            return Err(DbError::Conflict(
                "PO cannot receive in current status".into(),
            ));
        }

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        for rl in &lines {
            let line_uuid = parse_uuid(&rl.line_id)?;
            sqlx::query(
                "UPDATE purchase_order_lines SET \
                 quantity_received = LEAST(quantity, quantity_received + $1) \
                 WHERE id = $2 AND po_id = $3",
            )
            .bind(rl.quantity_received)
            .bind(line_uuid)
            .bind(id_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // Determine new status from received quantities
        let (total, received): (i64, i64) = sqlx::query_as(
            "SELECT SUM(quantity), SUM(quantity_received) FROM purchase_order_lines WHERE po_id = $1",
        )
        .bind(id_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let new_status = if received >= total {
            "received"
        } else if received > 0 {
            "partially_received"
        } else {
            "sent"
        };

        sqlx::query(
            "UPDATE purchase_orders SET status = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_status)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query("DELETE FROM purchase_orders WHERE id = $1 AND organization_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn add_line(
        pool: &PgPool,
        org_id: &str,
        po_id: &str,
        line: CreatePoLine,
    ) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let po_uuid = parse_uuid(po_id)?;

        // Verify ownership
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM purchase_orders WHERE id = $1 AND organization_id = $2)",
        )
        .bind(po_uuid)
        .bind(org_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        if !exists {
            return Err(DbError::NotFound);
        }

        let prod_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
        let sort_order: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM purchase_order_lines WHERE po_id = $1",
        )
        .bind(po_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "INSERT INTO purchase_order_lines \
             (po_id, product_id, description, quantity, unit_price, tax_rate, sort_order) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(po_uuid)
        .bind(prod_uuid)
        .bind(&line.description)
        .bind(line.quantity)
        .bind(line.unit_price)
        .bind(line.tax_rate)
        .bind(sort_order)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE purchase_orders SET updated_at = NOW() WHERE id = $1 AND organization_id = $2",
        )
        .bind(po_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, po_id).await
    }

    pub async fn approve(pool: &PgPool, org_id: &str, id: &str) -> Result<PurchaseOrder, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let rows = sqlx::query(
            "UPDATE purchase_orders SET status = 'approved', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows == 0 {
            let po = Self::get_by_id(pool, org_id, id).await?;
            return Err(DbError::Conflict(format!(
                "PO cannot be approved from status '{}'",
                po.status
            )));
        }

        Self::get_by_id(pool, org_id, id).await
    }
}
