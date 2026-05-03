use oxidebooks_core::models::{
    CreateInvoice, Invoice, InvoiceLine, InvoiceStatus, InvoiceType, UpdateInvoice,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_number: String,
    contact_id: Uuid,
    invoice_type: String,
    status: String,
    date: Date,
    due_date: Date,
    currency: String,
    notes: Option<String>,
    journal_entry_id: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct InvoiceLineRow {
    id: Uuid,
    invoice_id: Uuid,
    description: String,
    account_id: Option<Uuid>,
    quantity: i64,
    unit_price: i64,
    tax_rate: i64,
    sort_order: i32,
}

pub struct InvoiceRepo;

impl InvoiceRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<Invoice>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<InvoiceRow> = sqlx::query_as(
            "SELECT id, organization_id, invoice_number, contact_id, invoice_type, status, \
             date, due_date, currency, notes, journal_entry_id, created_at, updated_at \
             FROM invoices WHERE organization_id = $1 ORDER BY date DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut invoices = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            invoices.push(invoice_from_row(r, lines));
        }
        Ok(invoices)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Invoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: InvoiceRow = sqlx::query_as(
            "SELECT id, organization_id, invoice_number, contact_id, invoice_type, status, \
             date, due_date, currency, notes, journal_entry_id, created_at, updated_at \
             FROM invoices WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = Self::fetch_lines(pool, row.id).await?;
        Ok(invoice_from_row(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateInvoice,
    ) -> Result<Invoice, DbError> {
        input.validate()?;

        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = parse_uuid(&input.contact_id)?;
        let id = Uuid::new_v4();
        let invoice_type = input.invoice_type.to_string();
        let currency = input.currency.unwrap_or_else(|| "USD".to_string());

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
        let invoice_number =
            generate_invoice_number(&mut tx, org_uuid, &input.invoice_type).await?;

        sqlx::query(
            "INSERT INTO invoices \
             (id, organization_id, invoice_number, contact_id, invoice_type, \
              date, due_date, currency, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&invoice_number)
        .bind(contact_uuid)
        .bind(&invoice_type)
        .bind(input.date)
        .bind(input.due_date)
        .bind(&currency)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, line) in input.lines.iter().enumerate() {
            let line_id = Uuid::new_v4();
            let acct_uuid = line.account_id.as_deref().map(parse_uuid).transpose()?;
            let tax_rate = line.tax_rate.unwrap_or(0);
            sqlx::query(
                "INSERT INTO invoice_lines \
                 (id, invoice_id, description, account_id, quantity, unit_price, tax_rate, sort_order) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(line_id)
            .bind(id)
            .bind(&line.description)
            .bind(acct_uuid)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(tax_rate)
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
        input: UpdateInvoice,
    ) -> Result<Invoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        // Validate the status transition before writing anything.
        if let Some(ref new_status) = input.status {
            let current = Self::get_by_id(pool, org_id, id).await?;
            if !current.status.can_transition_to(new_status) {
                return Err(DbError::Conflict(format!(
                    "cannot transition invoice from '{}' to '{}'",
                    current.status, new_status
                )));
            }
            sqlx::query(
                "UPDATE invoices SET status = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(new_status.to_string())
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        if let Some(due_date) = input.due_date {
            sqlx::query(
                "UPDATE invoices SET due_date = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(due_date)
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        if let Some(ref notes) = input.notes {
            sqlx::query(
                "UPDATE invoices SET notes = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(notes)
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, org_id, id).await
    }

    async fn fetch_lines(pool: &PgPool, invoice_id: Uuid) -> Result<Vec<InvoiceLine>, DbError> {
        let rows: Vec<InvoiceLineRow> = sqlx::query_as(
            "SELECT id, invoice_id, description, account_id, quantity, unit_price, tax_rate, sort_order \
             FROM invoice_lines WHERE invoice_id = $1 ORDER BY sort_order",
        )
        .bind(invoice_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows
            .into_iter()
            .map(|r| InvoiceLine {
                id: r.id.to_string(),
                invoice_id: r.invoice_id.to_string(),
                description: r.description,
                account_id: r.account_id.map(|u| u.to_string()),
                quantity: r.quantity,
                unit_price: r.unit_price,
                tax_rate: r.tax_rate,
                sort_order: r.sort_order,
            })
            .collect())
    }
}

async fn generate_invoice_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    invoice_type: &InvoiceType,
) -> Result<String, DbError> {
    let prefix = match invoice_type {
        InvoiceType::Invoice => "INV",
        InvoiceType::Bill => "BILL",
    };
    let type_str = invoice_type.to_string();

    // Atomically increment (or insert) the counter for this org+type and return
    // the value just claimed.  ON CONFLICT ensures this is safe under concurrency.
    let next_val: i64 = sqlx::query_scalar(
        "INSERT INTO invoice_counters (organization_id, invoice_type, next_val)
         VALUES ($1, $2, 2)
         ON CONFLICT (organization_id, invoice_type)
         DO UPDATE SET next_val = invoice_counters.next_val + 1
         RETURNING next_val - 1",
    )
    .bind(org_id)
    .bind(&type_str)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;

    Ok(format!("{}-{:05}", prefix, next_val))
}

fn invoice_from_row(r: InvoiceRow, lines: Vec<InvoiceLine>) -> Invoice {
    Invoice {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_number: r.invoice_number,
        contact_id: r.contact_id.to_string(),
        invoice_type: match r.invoice_type.as_str() {
            "bill" => InvoiceType::Bill,
            _ => InvoiceType::Invoice,
        },
        status: InvoiceStatus::from_str(&r.status).unwrap_or(InvoiceStatus::Draft),
        date: r.date,
        due_date: r.due_date,
        currency: r.currency,
        notes: r.notes,
        lines,
        journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
