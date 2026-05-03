use oxidebooks_core::models::{
    ConvertPrToPo, CreatePurchaseRequisition, PrLine, PurchaseRequisition,
    UpdatePurchaseRequisition,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DbError;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct PrRow {
    id: Uuid,
    organization_id: Uuid,
    requester_id: Option<Uuid>,
    approver_id: Option<Uuid>,
    title: String,
    notes: Option<String>,
    status: String,
    total_amount: i64,
    approved_at: Option<time::OffsetDateTime>,
    rejected_at: Option<time::OffsetDateTime>,
    converted_po_id: Option<Uuid>,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct PrLineRow {
    id: Uuid,
    requisition_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i64,
    unit_price: i64,
    account_id: Option<Uuid>,
    sort_order: i32,
}

fn line_from_row(r: PrLineRow) -> PrLine {
    let line_total = r.quantity * r.unit_price;
    PrLine {
        id: r.id.to_string(),
        requisition_id: r.requisition_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        description: r.description,
        quantity: r.quantity,
        unit_price: r.unit_price,
        account_id: r.account_id.map(|u| u.to_string()),
        sort_order: r.sort_order,
        line_total,
    }
}

async fn fetch_lines<'e, E>(exec: E, pr_id: Uuid) -> Result<Vec<PrLine>, DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query_as::<_, PrLineRow>(
        "SELECT id, requisition_id, product_id, description, quantity,
                unit_price, account_id, sort_order
         FROM purchase_requisition_lines
         WHERE requisition_id = $1
         ORDER BY sort_order, id",
    )
    .bind(pr_id)
    .fetch_all(exec)
    .await?;
    Ok(rows.into_iter().map(line_from_row).collect())
}

fn pr_from_parts(r: PrRow, lines: Vec<PrLine>) -> PurchaseRequisition {
    PurchaseRequisition {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        requester_id: r.requester_id.map(|u| u.to_string()),
        approver_id: r.approver_id.map(|u| u.to_string()),
        title: r.title,
        notes: r.notes,
        status: r.status,
        total_amount: r.total_amount,
        approved_at: r.approved_at,
        rejected_at: r.rejected_at,
        converted_po_id: r.converted_po_id.map(|u| u.to_string()),
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const PR_COLS: &str = "id, organization_id, requester_id, approver_id, title, notes,
    status, total_amount, approved_at, rejected_at, converted_po_id, created_at, updated_at";

pub struct PurchaseRequisitionRepo;

impl PurchaseRequisitionRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<PurchaseRequisition>, DbError> {
        let org = parse_uuid(org_id)?;
        let rows = sqlx::query_as::<_, PrRow>(&format!(
            "SELECT {PR_COLS} FROM purchase_requisitions
             WHERE organization_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
             ORDER BY created_at DESC"
        ))
        .bind(org)
        .bind(status)
        .fetch_all(pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = fetch_lines(pool, r.id).await?;
            out.push(pr_from_parts(r, lines));
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let row = sqlx::query_as::<_, PrRow>(&format!(
            "SELECT {PR_COLS} FROM purchase_requisitions
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(pr_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(pr_from_parts(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        requester_id: &str,
        input: CreatePurchaseRequisition,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let req_id = parse_uuid(requester_id)?;

        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, PrRow>(&format!(
            "INSERT INTO purchase_requisitions
                (organization_id, requester_id, title, notes)
             VALUES ($1, $2, $3, $4)
             RETURNING {PR_COLS}"
        ))
        .bind(org)
        .bind(req_id)
        .bind(&input.title)
        .bind(&input.notes)
        .fetch_one(&mut *tx)
        .await?;

        let pr_id = row.id;
        let mut total: i64 = 0;
        let mut lines = Vec::new();

        for (i, line) in input.lines.into_iter().enumerate() {
            let prod = line.product_id.as_deref().map(parse_uuid).transpose()?;
            let acct = line.account_id.as_deref().map(parse_uuid).transpose()?;
            let line_total = line.quantity * line.unit_price;
            total += line_total;

            let lr = sqlx::query_as::<_, PrLineRow>(
                "INSERT INTO purchase_requisition_lines
                    (requisition_id, product_id, description, quantity, unit_price,
                     account_id, sort_order)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 RETURNING id, requisition_id, product_id, description, quantity,
                           unit_price, account_id, sort_order",
            )
            .bind(pr_id)
            .bind(prod)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(acct)
            .bind(line.sort_order.max(i as i32))
            .fetch_one(&mut *tx)
            .await?;
            lines.push(line_from_row(lr));
        }

        sqlx::query("UPDATE purchase_requisitions SET total_amount = $2 WHERE id = $1")
            .bind(pr_id)
            .bind(total)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let mut pr = pr_from_parts(row, lines);
        pr.total_amount = total;
        Ok(pr)
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdatePurchaseRequisition,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let row = sqlx::query_as::<_, PrRow>(&format!(
            "UPDATE purchase_requisitions
             SET title      = COALESCE($3, title),
                 notes      = COALESCE($4, notes),
                 updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'
             RETURNING {PR_COLS}"
        ))
        .bind(org)
        .bind(pr_id)
        .bind(&input.title)
        .bind(&input.notes)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(pr_from_parts(row, lines))
    }

    pub async fn submit(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let row = sqlx::query_as::<_, PrRow>(&format!(
            "UPDATE purchase_requisitions
             SET status = 'submitted', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'
             RETURNING {PR_COLS}"
        ))
        .bind(org)
        .bind(pr_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::Conflict(
            "requisition is not in draft status".into(),
        ))?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(pr_from_parts(row, lines))
    }

    pub async fn approve(
        pool: &PgPool,
        org_id: &str,
        approver_id: &str,
        id: &str,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let approver = parse_uuid(approver_id)?;
        let row = sqlx::query_as::<_, PrRow>(&format!(
            "UPDATE purchase_requisitions
             SET status      = 'approved',
                 approver_id = $3,
                 approved_at = now(),
                 updated_at  = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'submitted'
             RETURNING {PR_COLS}"
        ))
        .bind(org)
        .bind(pr_id)
        .bind(approver)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::Conflict(
            "requisition is not in submitted status".into(),
        ))?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(pr_from_parts(row, lines))
    }

    pub async fn reject(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let row = sqlx::query_as::<_, PrRow>(&format!(
            "UPDATE purchase_requisitions
             SET status      = 'rejected',
                 rejected_at = now(),
                 updated_at  = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'submitted'
             RETURNING {PR_COLS}"
        ))
        .bind(org)
        .bind(pr_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::Conflict(
            "requisition is not in submitted status".into(),
        ))?;
        let lines = fetch_lines(pool, row.id).await?;
        Ok(pr_from_parts(row, lines))
    }

    /// Convert an approved requisition into a purchase order.
    pub async fn convert_to_po(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: ConvertPrToPo,
    ) -> Result<PurchaseRequisition, DbError> {
        let org = parse_uuid(org_id)?;
        let pr_id = parse_uuid(id)?;
        let vendor = parse_uuid(&input.vendor_id)?;

        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, PrRow>(&format!(
            "SELECT {PR_COLS} FROM purchase_requisitions
             WHERE organization_id = $1 AND id = $2 AND status = 'approved'
             FOR UPDATE"
        ))
        .bind(org)
        .bind(pr_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DbError::Conflict(
            "requisition is not in approved status".into(),
        ))?;

        let lines = fetch_lines(pool, row.id).await?;

        // Generate a PO number
        let next_val: i64 = sqlx::query_scalar(
            "INSERT INTO po_counters (organization_id, next_val)
             VALUES ($1, 2)
             ON CONFLICT (organization_id) DO UPDATE
                SET next_val = po_counters.next_val + 1
             RETURNING next_val - 1",
        )
        .bind(org)
        .fetch_one(&mut *tx)
        .await?;
        let po_number = format!("PO-{next_val:05}");

        // Create the PO
        let po_id: Uuid = sqlx::query_scalar(
            "INSERT INTO purchase_orders
                (organization_id, po_number, contact_id, order_date, status, notes)
             VALUES ($1, $2, $3, CURRENT_DATE, 'draft', $4)
             RETURNING id",
        )
        .bind(org)
        .bind(&po_number)
        .bind(vendor)
        .bind(&row.notes)
        .fetch_one(&mut *tx)
        .await?;

        // Copy lines to PO lines
        for line in &lines {
            let prod = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO purchase_order_lines
                    (po_id, product_id, description, quantity, unit_price, sort_order)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(po_id)
            .bind(prod)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        // Mark PR as converted
        let updated = sqlx::query_as::<_, PrRow>(&format!(
            "UPDATE purchase_requisitions
             SET status          = 'converted',
                 converted_po_id = $3,
                 updated_at      = now()
             WHERE id = $1 AND organization_id = $2
             RETURNING {PR_COLS}"
        ))
        .bind(pr_id)
        .bind(org)
        .bind(po_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(pr_from_parts(updated, lines))
    }
}
