use oxidebooks_core::models::{
    CreateInvoice, Invoice, InvoiceFilters, InvoiceLine, InvoiceStatus, InvoiceType, UpdateInvoice,
};
use oxidebooks_core::pagination::{encode_cursor, PageParams};
use rust_decimal::Decimal;
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
    exchange_rate: Decimal,
    doc_number: Option<String>,
    notes: Option<String>,
    expiry_date: Option<Date>,
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
    discount_pct: i64,
    sort_order: i32,
    product_id: Option<Uuid>,
}

pub struct InvoiceRepo;

impl InvoiceRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        page: &PageParams,
        filters: &InvoiceFilters,
    ) -> Result<(Vec<Invoice>, Option<String>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let limit = page.limit_clamped();
        let cursor = page.decode_cursor();

        let contact_uuid = filters.contact_id.as_deref().map(parse_uuid).transpose()?;

        let rows: Vec<InvoiceRow> = if let Some(c) = cursor {
            let cursor_ts = time::OffsetDateTime::parse(
                &c.created_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| DbError::Conflict("invalid cursor".into()))?;
            let cursor_id = parse_uuid(&c.id)?;
            sqlx::query_as(
                "SELECT id, organization_id, invoice_number, contact_id, invoice_type, status, \
                 date, due_date, currency, exchange_rate, doc_number, notes, expiry_date, journal_entry_id, created_at, updated_at \
                 FROM invoices \
                 WHERE organization_id = $1 \
                   AND (created_at, id) > ($2, $3) \
                   AND ($4::text IS NULL OR status = $4) \
                   AND ($5::text IS NULL OR invoice_type = $5) \
                   AND ($6::uuid IS NULL OR contact_id = $6) \
                   AND ($7::date IS NULL OR date >= $7) \
                   AND ($8::date IS NULL OR date <= $8) \
                 ORDER BY created_at ASC, id ASC LIMIT $9",
            )
            .bind(org_uuid)
            .bind(cursor_ts)
            .bind(cursor_id)
            .bind(filters.status.as_deref())
            .bind(filters.invoice_type.as_deref())
            .bind(contact_uuid)
            .bind(filters.from)
            .bind(filters.to)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        } else {
            sqlx::query_as(
                "SELECT id, organization_id, invoice_number, contact_id, invoice_type, status, \
                 date, due_date, currency, exchange_rate, doc_number, notes, expiry_date, journal_entry_id, created_at, updated_at \
                 FROM invoices \
                 WHERE organization_id = $1 \
                   AND ($2::text IS NULL OR status = $2) \
                   AND ($3::text IS NULL OR invoice_type = $3) \
                   AND ($4::uuid IS NULL OR contact_id = $4) \
                   AND ($5::date IS NULL OR date >= $5) \
                   AND ($6::date IS NULL OR date <= $6) \
                 ORDER BY created_at ASC, id ASC LIMIT $7",
            )
            .bind(org_uuid)
            .bind(filters.status.as_deref())
            .bind(filters.invoice_type.as_deref())
            .bind(contact_uuid)
            .bind(filters.from)
            .bind(filters.to)
            .bind(limit + 1)
            .fetch_all(pool)
            .await
            .map_err(map_sqlx_err)?
        };

        let has_next = rows.len() as i64 > limit;
        let mut rows = rows;
        if has_next {
            rows.pop();
        }
        let next_cursor = if has_next {
            rows.last()
                .map(|r| encode_cursor(r.created_at, &r.id.to_string()))
        } else {
            None
        };
        let mut invoices = Vec::with_capacity(rows.len());
        for r in rows {
            let lines = Self::fetch_lines(pool, r.id).await?;
            invoices.push(invoice_from_row(r, lines));
        }
        Ok((invoices, next_cursor))
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<Invoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: InvoiceRow = sqlx::query_as(
            "SELECT id, organization_id, invoice_number, contact_id, invoice_type, status, \
             date, due_date, currency, notes, expiry_date, journal_entry_id, created_at, updated_at \
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
        let exchange_rate = input.exchange_rate.unwrap_or(Decimal::ONE);

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
        let invoice_number =
            generate_invoice_number(&mut tx, org_uuid, &input.invoice_type).await?;

        sqlx::query(
            "INSERT INTO invoices \
             (id, organization_id, invoice_number, contact_id, invoice_type, \
              date, due_date, currency, exchange_rate, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(&invoice_number)
        .bind(contact_uuid)
        .bind(&invoice_type)
        .bind(input.date)
        .bind(input.due_date)
        .bind(&currency)
        .bind(exchange_rate)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, line) in input.lines.iter().enumerate() {
            let line_id = Uuid::new_v4();
            let acct_uuid = line.account_id.as_deref().map(parse_uuid).transpose()?;
            let tax_rate = line.tax_rate.unwrap_or(0);
            let prod_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO invoice_lines \
                 (id, invoice_id, description, account_id, quantity, unit_price, \
                  tax_rate, discount_pct, sort_order, product_id) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(line_id)
            .bind(id)
            .bind(&line.description)
            .bind(acct_uuid)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(tax_rate)
            .bind(line.discount_pct)
            .bind(i as i32)
            .bind(prod_uuid)
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

        if let Some(expiry_date) = input.expiry_date {
            sqlx::query(
                "UPDATE invoices SET expiry_date = $1, updated_at = NOW() \
                 WHERE id = $2 AND organization_id = $3",
            )
            .bind(expiry_date)
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get_by_id(pool, org_id, id).await
    }

    /// Transition a quote to accepted/declined/expired status.
    pub async fn update_quote_status(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        new_status: &str,
    ) -> Result<Invoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let quote = Self::get_by_id(pool, org_id, id).await?;
        if quote.invoice_type != InvoiceType::Quote {
            return Err(DbError::Conflict("only quotes support this action".into()));
        }
        let valid_transitions: &[&str] = match new_status {
            "accepted" | "declined" => &["draft", "sent"],
            "expired" => &["sent"],
            _ => {
                return Err(DbError::Conflict(format!(
                    "invalid quote status '{new_status}'"
                )))
            }
        };
        if !valid_transitions.contains(&quote.status.to_string().as_str()) {
            return Err(DbError::Conflict(format!(
                "cannot transition quote from '{}' to '{new_status}'",
                quote.status
            )));
        }

        sqlx::query(
            "UPDATE invoices SET status = $1, updated_at = NOW() \
             WHERE id = $2 AND organization_id = $3",
        )
        .bind(new_status)
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, id).await
    }

    /// Convert an accepted quote into a new invoice. Marks the quote as `invoiced`.
    pub async fn convert_quote(
        pool: &PgPool,
        org_id: &str,
        quote_id: &str,
    ) -> Result<Invoice, DbError> {
        let quote = Self::get_by_id(pool, org_id, quote_id).await?;

        if quote.invoice_type != InvoiceType::Quote {
            return Err(DbError::Conflict("only quotes can be converted".into()));
        }
        if quote.status != InvoiceStatus::Accepted {
            return Err(DbError::Conflict(
                "quote must be in 'accepted' status to convert".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let quote_uuid = parse_uuid(quote_id)?;
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Mark quote as invoiced
        sqlx::query(
            "UPDATE invoices SET status = 'invoiced', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(quote_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Create new invoice from quote data
        let invoice_number =
            generate_invoice_number(&mut tx, org_uuid, &InvoiceType::Invoice).await?;
        let new_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO invoices \
             (id, organization_id, invoice_number, contact_id, invoice_type, \
              date, due_date, currency, notes) \
             VALUES ($1,$2,$3,$4,'invoice',$5,$6,$7,$8)",
        )
        .bind(new_id)
        .bind(org_uuid)
        .bind(&invoice_number)
        .bind(parse_uuid(&quote.contact_id)?)
        .bind(quote.date)
        .bind(quote.due_date)
        .bind(&quote.currency)
        .bind(&quote.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for (i, line) in quote.lines.iter().enumerate() {
            let line_id = Uuid::new_v4();
            let acct_uuid = line.account_id.as_deref().map(parse_uuid).transpose()?;
            let prod_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            sqlx::query(
                "INSERT INTO invoice_lines \
                 (id, invoice_id, description, account_id, quantity, unit_price, \
                  tax_rate, discount_pct, sort_order, product_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(line_id)
            .bind(new_id)
            .bind(&line.description)
            .bind(acct_uuid)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(line.discount_pct)
            .bind(i as i32)
            .bind(prod_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &new_id.to_string()).await
    }

    /// Apply a credit note against an invoice. Creates a payment of type 'credit_note'.
    pub async fn apply_credit(
        pool: &PgPool,
        org_id: &str,
        credit_note_id: &str,
        target_invoice_id: &str,
        amount: i64,
    ) -> Result<Invoice, DbError> {
        use time::OffsetDateTime;

        let cn = Self::get_by_id(pool, org_id, credit_note_id).await?;
        if cn.invoice_type != InvoiceType::CreditNote {
            return Err(DbError::Conflict("source must be a credit note".into()));
        }
        if cn.status != InvoiceStatus::Sent {
            return Err(DbError::Conflict(
                "credit note must be in 'sent' status to apply".into(),
            ));
        }

        let org_uuid = parse_uuid(org_id)?;
        let cn_uuid = parse_uuid(credit_note_id)?;
        let inv_uuid = parse_uuid(target_invoice_id)?;
        let today = OffsetDateTime::now_utc().date();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;

        // Record payment on the target invoice
        sqlx::query(
            "INSERT INTO payments \
             (organization_id, invoice_id, amount, payment_date, method, notes) \
             VALUES ($1,$2,$3,$4,'credit_note','Applied from credit note')",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(amount)
        .bind(today)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // Mark credit note as applied
        sqlx::query(
            "UPDATE invoices SET status = 'applied', updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(cn_uuid)
        .bind(org_uuid)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;

        // Sync the target invoice status
        crate::repos::payments::PaymentRepo::sync_invoice_status(pool, org_uuid, inv_uuid).await?;
        Self::get_by_id(pool, org_id, target_invoice_id).await
    }

    async fn fetch_lines(pool: &PgPool, invoice_id: Uuid) -> Result<Vec<InvoiceLine>, DbError> {
        let rows: Vec<InvoiceLineRow> = sqlx::query_as(
            "SELECT id, invoice_id, description, account_id, quantity, unit_price, \
             tax_rate, discount_pct, sort_order, product_id \
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
                discount_pct: r.discount_pct,
                sort_order: r.sort_order,
                product_id: r.product_id.map(|u| u.to_string()),
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
        InvoiceType::Quote => "QUO",
        InvoiceType::CreditNote => "CN",
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
            "quote" => InvoiceType::Quote,
            "credit_note" => InvoiceType::CreditNote,
            _ => InvoiceType::Invoice,
        },
        status: InvoiceStatus::from_str(&r.status).unwrap_or(InvoiceStatus::Draft),
        date: r.date,
        due_date: r.due_date,
        currency: r.currency,
        exchange_rate: r.exchange_rate,
        doc_number: r.doc_number,
        notes: r.notes,
        expiry_date: r.expiry_date,
        lines,
        journal_entry_id: r.journal_entry_id.map(|u| u.to_string()),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
