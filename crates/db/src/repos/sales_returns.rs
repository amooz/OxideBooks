use oxidebooks_core::models::{
    ApproveSalesReturn, CreateCreditNote, CreateSalesReturn, InventoryAdjustment,
    ReceiveSalesReturn, SalesReturn, SalesReturnLine,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::{CreditNoteRepo, InventoryRepo};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ReturnRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Option<Uuid>,
    contact_id: Option<Uuid>,
    rma_number: String,
    status: String,
    reason: Option<String>,
    notes: Option<String>,
    credit_note_id: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
    received_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ReturnLineRow {
    id: Uuid,
    return_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i64,
    unit_price: i64,
    restock: bool,
    created_at: OffsetDateTime,
}

impl From<ReturnLineRow> for SalesReturnLine {
    fn from(r: ReturnLineRow) -> Self {
        SalesReturnLine {
            id: r.id.to_string(),
            return_id: r.return_id.to_string(),
            product_id: r.product_id.map(|u| u.to_string()),
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            restock: r.restock,
            created_at: r.created_at,
        }
    }
}

const RETURN_COLS: &str = "id, organization_id, invoice_id, contact_id, rma_number, status, \
     reason, notes, credit_note_id, approved_at, received_at, created_at, updated_at";

async fn fetch_lines(pool: &PgPool, return_id: Uuid) -> Result<Vec<SalesReturnLine>, DbError> {
    let rows: Vec<ReturnLineRow> = sqlx::query_as(
        "SELECT id, return_id, product_id, description, quantity, unit_price, restock, created_at \
         FROM sales_return_lines WHERE return_id = $1 ORDER BY created_at ASC",
    )
    .bind(return_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(SalesReturnLine::from).collect())
}

fn to_return(r: ReturnRow, lines: Vec<SalesReturnLine>) -> SalesReturn {
    SalesReturn {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        contact_id: r.contact_id.map(|u| u.to_string()),
        rma_number: r.rma_number,
        status: r.status,
        reason: r.reason,
        notes: r.notes,
        credit_note_id: r.credit_note_id.map(|u| u.to_string()),
        approved_at: r.approved_at,
        received_at: r.received_at,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct SalesReturnRepo;

impl SalesReturnRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<SalesReturn>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ReturnRow> = sqlx::query_as(&format!(
            "SELECT {RETURN_COLS} FROM sales_returns \
             WHERE organization_id = $1 \
               AND ($2::text IS NULL OR status = $2) \
             ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut returns = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let lines = fetch_lines(pool, id).await?;
            returns.push(to_return(row, lines));
        }
        Ok(returns)
    }

    pub async fn get(pool: &PgPool, org_id: &str, return_id: &str) -> Result<SalesReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let row: ReturnRow = sqlx::query_as(&format!(
            "SELECT {RETURN_COLS} FROM sales_returns \
             WHERE id = $1 AND organization_id = $2"
        ))
        .bind(r_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, r_uuid).await?;
        Ok(to_return(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateSalesReturn,
    ) -> Result<SalesReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let invoice_uuid = input.invoice_id.as_deref().map(parse_uuid).transpose()?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;

        // Validate invoice belongs to this org if provided.
        if let Some(inv_uuid) = invoice_uuid {
            let exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM invoices WHERE id = $1 AND organization_id = $2")
                    .bind(inv_uuid)
                    .bind(org_uuid)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            if exists.is_none() {
                return Err(DbError::NotFound);
            }
        }

        // Generate RMA number.
        let rma_number = format!("RMA-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO sales_returns \
             (id, organization_id, invoice_id, contact_id, rma_number, reason, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(invoice_uuid)
        .bind(contact_uuid)
        .bind(&rma_number)
        .bind(&input.reason)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        for line in &input.lines {
            let product_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO sales_return_lines \
                 (return_id, product_id, description, quantity, unit_price, restock) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(id)
            .bind(product_uuid)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.restock)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get(pool, org_id, &id.to_string()).await
    }

    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        return_id: &str,
        input: ApproveSalesReturn,
    ) -> Result<SalesReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let rows_affected = sqlx::query(
            "UPDATE sales_returns \
             SET status = 'approved', approved_at = NOW(), \
                 notes = COALESCE($3, notes), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'requested'",
        )
        .bind(r_uuid)
        .bind(org_uuid)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT status FROM sales_returns WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "return cannot be approved from status '{s}'"
                ))),
            };
        }

        Self::get(pool, org_id, return_id).await
    }

    /// Receive a return: restock inventory for lines with restock=true, then generate credit note.
    pub async fn receive(
        pool: &PgPool,
        org_id: &str,
        return_id: &str,
        input: ReceiveSalesReturn,
    ) -> Result<SalesReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let rows_affected = sqlx::query(
            "UPDATE sales_returns \
             SET status = 'received', received_at = NOW(), \
                 notes = COALESCE($3, notes), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'approved'",
        )
        .bind(r_uuid)
        .bind(org_uuid)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT status FROM sales_returns WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "return cannot be received from status '{s}'"
                ))),
            };
        }

        // Restock inventory for each line that has restock=true and a product_id.
        let lines = fetch_lines(pool, r_uuid).await?;
        for line in &lines {
            if line.restock {
                if let Some(product_id) = &line.product_id {
                    let qty = line.quantity / 100; // stored as qty×100
                    let _ = InventoryRepo::adjust(
                        pool,
                        org_id,
                        product_id,
                        InventoryAdjustment {
                            quantity: qty,
                            unit_cost: None,
                            notes: Some(format!("RMA restock: {return_id}")),
                        },
                    )
                    .await;
                    // Ignore NotFound — product may not be tracked in inventory.
                }
            }
        }

        // Auto-generate credit note.
        let sr = Self::get(pool, org_id, return_id).await?;
        let total: i64 = sr
            .lines
            .iter()
            .map(|l| l.quantity * l.unit_price / 100)
            .sum();

        if total > 0 {
            let contact_id = sr.contact_id.clone();
            let cn = CreditNoteRepo::create(
                pool,
                org_id,
                CreateCreditNote {
                    contact_id,
                    note_date: time::OffsetDateTime::now_utc().date(),
                    reference: Some(sr.rma_number.clone()),
                    description: format!("Credit for return {}", &sr.rma_number),
                    amount: total,
                },
            )
            .await?;

            let cn_uuid = parse_uuid(&cn.id)?;
            sqlx::query(
                "UPDATE sales_returns SET credit_note_id = $3, status = 'closed', updated_at = NOW() \
                 WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .bind(cn_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get(pool, org_id, return_id).await
    }
}
