use oxidebooks_core::models::{
    CreateDeferredCharge, DeferredCharge, InvoiceDeferredCharges, UpdateDeferredCharge,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::InvoiceRepo;
use oxidebooks_core::models::{CreateInvoice, CreateInvoiceLine, InvoiceType};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct ChargeRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Uuid,
    account_id: Option<Uuid>,
    description: String,
    charge_date: Date,
    amount: i64,
    tax_rate: i64,
    status: String,
    invoice_id: Option<Uuid>,
    invoiced_at: Option<OffsetDateTime>,
    memo: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn from_row(r: ChargeRow) -> DeferredCharge {
    DeferredCharge {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.to_string(),
        account_id: r.account_id.map(|u| u.to_string()),
        description: r.description,
        charge_date: r.charge_date,
        amount: r.amount,
        tax_rate: r.tax_rate,
        status: r.status,
        invoice_id: r.invoice_id.map(|u| u.to_string()),
        invoiced_at: r.invoiced_at,
        memo: r.memo,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

const COLS: &str = "id, organization_id, contact_id, account_id, description, charge_date,
     amount, tax_rate, status, invoice_id, invoiced_at, memo, created_at, updated_at";

pub struct DeferredChargeRepo;

impl DeferredChargeRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        contact_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<DeferredCharge>, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = contact_id.map(parse_uuid).transpose()?;
        let rows: Vec<ChargeRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deferred_charges
             WHERE organization_id = $1
               AND ($2::UUID IS NULL OR contact_id = $2)
               AND ($3::TEXT IS NULL OR status = $3)
             ORDER BY charge_date DESC, created_at DESC"
        ))
        .bind(org)
        .bind(contact)
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<DeferredCharge, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let row: ChargeRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deferred_charges WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(cid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateDeferredCharge,
    ) -> Result<DeferredCharge, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = parse_uuid(&input.contact_id)?;
        let account = input.account_id.as_deref().map(parse_uuid).transpose()?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO deferred_charges
                (organization_id, contact_id, account_id, description, charge_date,
                 amount, tax_rate, memo)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
        )
        .bind(org)
        .bind(contact)
        .bind(account)
        .bind(&input.description)
        .bind(input.charge_date)
        .bind(input.amount)
        .bind(input.tax_rate)
        .bind(&input.memo)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateDeferredCharge,
    ) -> Result<DeferredCharge, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE deferred_charges
             SET description = COALESCE($3, description),
                 amount      = COALESCE($4, amount),
                 tax_rate    = COALESCE($5, tax_rate),
                 memo        = COALESCE($6, memo),
                 updated_at  = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'",
        )
        .bind(org)
        .bind(cid)
        .bind(&input.description)
        .bind(input.amount)
        .bind(input.tax_rate)
        .bind(&input.memo)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "charge not found or already invoiced/voided".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn void(pool: &PgPool, org_id: &str, id: &str) -> Result<DeferredCharge, DbError> {
        let org = parse_uuid(org_id)?;
        let cid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE deferred_charges SET status = 'void', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'",
        )
        .bind(org)
        .bind(cid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict("charge must be pending to void".into()));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Convert one or more pending charges for the same contact into a single invoice.
    pub async fn invoice_charges(
        pool: &PgPool,
        org_id: &str,
        primary_id: &str,
        input: InvoiceDeferredCharges,
    ) -> Result<oxidebooks_core::models::Invoice, DbError> {
        let org = parse_uuid(org_id)?;
        let primary_uuid = parse_uuid(primary_id)?;

        // Fetch primary charge
        let primary: ChargeRow = sqlx::query_as(&format!(
            "SELECT {COLS} FROM deferred_charges
             WHERE organization_id = $1 AND id = $2 AND status = 'pending'"
        ))
        .bind(org)
        .bind(primary_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let contact_id = primary.contact_id;

        // Build list of all charge IDs to include
        let mut all_ids: Vec<Uuid> = vec![primary_uuid];
        for extra in &input.additional_ids {
            let eid = parse_uuid(extra)?;
            all_ids.push(eid);
        }

        // Fetch all charges (verify same contact and pending)
        let mut charge_rows: Vec<ChargeRow> = Vec::new();
        for cid in &all_ids {
            let row: ChargeRow = sqlx::query_as(&format!(
                "SELECT {COLS} FROM deferred_charges
                 WHERE organization_id = $1 AND id = $2 AND status = 'pending'
                   AND contact_id = $3"
            ))
            .bind(org)
            .bind(cid)
            .bind(contact_id)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx_err)?
            .ok_or(DbError::NotFound)?;
            charge_rows.push(row);
        }

        let lines: Vec<CreateInvoiceLine> = charge_rows
            .iter()
            .map(|c| CreateInvoiceLine {
                description: c.description.clone(),
                account_id: c.account_id.map(|u| u.to_string()),
                quantity: 100, // quantity × 100 = 1 unit
                unit_price: c.amount,
                tax_rate: Some(c.tax_rate),
                discount_pct: 0,
                product_id: None,
            })
            .collect();

        let inv_input = CreateInvoice {
            contact_id: contact_id.to_string(),
            invoice_type: InvoiceType::Invoice,
            date: input.invoice_date,
            due_date: input.due_date,
            currency: None,
            exchange_rate: None,
            notes: None,
            global_discount_pct: 0,
            lines,
        };

        let invoice = InvoiceRepo::create(pool, org_id, inv_input).await?;

        // Mark all charges as invoiced
        let inv_uuid = parse_uuid(&invoice.id)?;
        let now = time::OffsetDateTime::now_utc();
        for cid in &all_ids {
            sqlx::query(
                "UPDATE deferred_charges
                 SET status = 'invoiced', invoice_id = $2, invoiced_at = $3, updated_at = now()
                 WHERE id = $1",
            )
            .bind(cid)
            .bind(inv_uuid)
            .bind(now)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Ok(invoice)
    }
}
