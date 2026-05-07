use oxidebooks_core::models::{
    ApprovePurchaseReturn, CreatePurchaseReturn, CreateVendorCredit, CreateVendorCreditLine,
    InventoryAdjustment, PurchaseReturn, PurchaseReturnLine, ShipPurchaseReturn,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::{InventoryRepo, VendorCreditRepo};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ReturnRow {
    id: Uuid,
    organization_id: Uuid,
    bill_id: Option<Uuid>,
    contact_id: Option<Uuid>,
    rma_number: String,
    status: String,
    reason: Option<String>,
    notes: Option<String>,
    vendor_credit_id: Option<Uuid>,
    approved_at: Option<OffsetDateTime>,
    shipped_at: Option<OffsetDateTime>,
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
    created_at: OffsetDateTime,
}

impl From<ReturnLineRow> for PurchaseReturnLine {
    fn from(r: ReturnLineRow) -> Self {
        PurchaseReturnLine {
            id: r.id.to_string(),
            return_id: r.return_id.to_string(),
            product_id: r.product_id.map(|u| u.to_string()),
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            created_at: r.created_at,
        }
    }
}

const RETURN_COLS: &str = "id, organization_id, bill_id, contact_id, rma_number, status, \
     reason, notes, vendor_credit_id, approved_at, shipped_at, created_at, updated_at";

async fn fetch_lines(pool: &PgPool, return_id: Uuid) -> Result<Vec<PurchaseReturnLine>, DbError> {
    let rows: Vec<ReturnLineRow> = sqlx::query_as(
        "SELECT id, return_id, product_id, description, quantity, unit_price, created_at \
         FROM purchase_return_lines WHERE return_id = $1 ORDER BY created_at ASC",
    )
    .bind(return_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(PurchaseReturnLine::from).collect())
}

fn to_return(r: ReturnRow, lines: Vec<PurchaseReturnLine>) -> PurchaseReturn {
    PurchaseReturn {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        bill_id: r.bill_id.map(|u| u.to_string()),
        contact_id: r.contact_id.map(|u| u.to_string()),
        rma_number: r.rma_number,
        status: r.status,
        reason: r.reason,
        notes: r.notes,
        vendor_credit_id: r.vendor_credit_id.map(|u| u.to_string()),
        approved_at: r.approved_at,
        shipped_at: r.shipped_at,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct PurchaseReturnRepo;

impl PurchaseReturnRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<PurchaseReturn>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ReturnRow> = sqlx::query_as(&format!(
            "SELECT {RETURN_COLS} FROM purchase_returns \
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

    pub async fn get(
        pool: &PgPool,
        org_id: &str,
        return_id: &str,
    ) -> Result<PurchaseReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let row: ReturnRow = sqlx::query_as(&format!(
            "SELECT {RETURN_COLS} FROM purchase_returns \
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
        input: CreatePurchaseReturn,
    ) -> Result<PurchaseReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let bill_uuid = input.bill_id.as_deref().map(parse_uuid).transpose()?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;

        // Validate bill belongs to this org if provided.
        if let Some(b_uuid) = bill_uuid {
            let exists: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM vendor_bills WHERE id = $1 AND organization_id = $2",
            )
            .bind(b_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            if exists.is_none() {
                return Err(DbError::NotFound);
            }
        }

        let rma_number = format!("PRMA-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO purchase_returns \
             (id, organization_id, bill_id, contact_id, rma_number, reason, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(bill_uuid)
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
                "INSERT INTO purchase_return_lines \
                 (return_id, product_id, description, quantity, unit_price) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(product_uuid)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
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
        input: ApprovePurchaseReturn,
    ) -> Result<PurchaseReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let rows_affected = sqlx::query(
            "UPDATE purchase_returns \
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
                "SELECT status FROM purchase_returns WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "purchase return cannot be approved from status '{s}'"
                ))),
            };
        }

        Self::get(pool, org_id, return_id).await
    }

    /// Ship a return: reduce inventory for lines with a product, then generate vendor credit.
    pub async fn ship(
        pool: &PgPool,
        org_id: &str,
        return_id: &str,
        input: ShipPurchaseReturn,
    ) -> Result<PurchaseReturn, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let r_uuid = parse_uuid(return_id)?;

        let rows_affected = sqlx::query(
            "UPDATE purchase_returns \
             SET status = 'shipped', shipped_at = NOW(), \
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
                "SELECT status FROM purchase_returns WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "purchase return cannot be shipped from status '{s}'"
                ))),
            };
        }

        // Reduce inventory for each line that has a product.
        let lines = fetch_lines(pool, r_uuid).await?;
        for line in &lines {
            if let Some(product_id) = &line.product_id {
                let qty = -(line.quantity / 100);
                InventoryRepo::adjust(
                    pool,
                    org_id,
                    product_id,
                    InventoryAdjustment {
                        quantity: qty,
                        unit_cost: None,
                        notes: Some(format!("Purchase return shipped: {return_id}")),
                    },
                )
                .await?;
            }
        }

        // Auto-generate vendor credit.
        let pr = Self::get(pool, org_id, return_id).await?;
        let total: i64 = pr
            .lines
            .iter()
            .map(|l| l.quantity * l.unit_price / 100)
            .sum();

        if total > 0 {
            let vc_lines: Vec<CreateVendorCreditLine> = pr
                .lines
                .iter()
                .map(|l| CreateVendorCreditLine {
                    account_id: None,
                    description: Some(l.description.clone()),
                    quantity: l.quantity,
                    unit_price: l.unit_price,
                    tax_rate: 0,
                    sort_order: 0,
                })
                .collect();

            let vc = VendorCreditRepo::create(
                pool,
                org_id,
                CreateVendorCredit {
                    contact_id: pr.contact_id.clone(),
                    credit_date: time::OffsetDateTime::now_utc().date(),
                    reference: Some(pr.rma_number.clone()),
                    memo: Some(format!(
                        "Vendor credit for purchase return {}",
                        &pr.rma_number
                    )),
                    lines: vc_lines,
                },
            )
            .await?;

            let vc_uuid = parse_uuid(&vc.id)?;
            sqlx::query(
                "UPDATE purchase_returns \
                 SET vendor_credit_id = $3, status = 'closed', updated_at = NOW() \
                 WHERE id = $1 AND organization_id = $2",
            )
            .bind(r_uuid)
            .bind(org_uuid)
            .bind(vc_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get(pool, org_id, return_id).await
    }
}
