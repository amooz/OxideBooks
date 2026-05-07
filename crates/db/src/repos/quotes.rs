use oxidebooks_core::models::{
    ConvertQuoteToInvoice, CreateInvoice, CreateInvoiceLine, CreateQuote, InvoiceType, Quote,
    QuoteLine, UpdateQuote,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::{DocSequenceRepo, InvoiceRepo};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct QuoteRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: Option<Uuid>,
    quote_number: String,
    status: String,
    issue_date: Date,
    expiry_date: Option<Date>,
    currency: String,
    exchange_rate: Decimal,
    notes: Option<String>,
    terms: Option<String>,
    sent_at: Option<OffsetDateTime>,
    accepted_at: Option<OffsetDateTime>,
    declined_at: Option<OffsetDateTime>,
    invoiced_at: Option<OffsetDateTime>,
    converted_invoice_id: Option<Uuid>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct QuoteLineRow {
    id: Uuid,
    quote_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i64,
    unit_price: i64,
    discount_pct: i64,
    tax_rate: i64,
    sort_order: i32,
    created_at: OffsetDateTime,
}

impl From<QuoteLineRow> for QuoteLine {
    fn from(r: QuoteLineRow) -> Self {
        QuoteLine {
            id: r.id.to_string(),
            quote_id: r.quote_id.to_string(),
            product_id: r.product_id.map(|u| u.to_string()),
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            discount_pct: r.discount_pct,
            tax_rate: r.tax_rate,
            sort_order: r.sort_order,
            created_at: r.created_at,
        }
    }
}

const QUOTE_COLS: &str = "id, organization_id, contact_id, quote_number, status, issue_date, \
     expiry_date, currency, exchange_rate, notes, terms, sent_at, accepted_at, declined_at, \
     invoiced_at, converted_invoice_id, created_at, updated_at";

async fn fetch_lines(pool: &PgPool, quote_id: Uuid) -> Result<Vec<QuoteLine>, DbError> {
    let rows: Vec<QuoteLineRow> = sqlx::query_as(
        "SELECT id, quote_id, product_id, description, quantity, unit_price, \
         discount_pct, tax_rate, sort_order, created_at \
         FROM quote_lines WHERE quote_id = $1 ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(quote_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(QuoteLine::from).collect())
}

fn to_quote(r: QuoteRow, lines: Vec<QuoteLine>) -> Quote {
    Quote {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id.map(|u| u.to_string()),
        quote_number: r.quote_number,
        status: r.status,
        issue_date: r.issue_date,
        expiry_date: r.expiry_date,
        currency: r.currency,
        exchange_rate: r.exchange_rate,
        notes: r.notes,
        terms: r.terms,
        sent_at: r.sent_at,
        accepted_at: r.accepted_at,
        declined_at: r.declined_at,
        invoiced_at: r.invoiced_at,
        converted_invoice_id: r.converted_invoice_id.map(|u| u.to_string()),
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct QuoteRepo;

impl QuoteRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        contact_id: Option<&str>,
    ) -> Result<Vec<Quote>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = contact_id.map(parse_uuid).transpose()?;

        let rows: Vec<QuoteRow> = sqlx::query_as(&format!(
            "SELECT {QUOTE_COLS} FROM quotes \
             WHERE organization_id = $1 \
               AND ($2::text IS NULL OR status = $2) \
               AND ($3::uuid IS NULL OR contact_id = $3) \
             ORDER BY created_at DESC"
        ))
        .bind(org_uuid)
        .bind(status)
        .bind(contact_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut quotes = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let lines = fetch_lines(pool, id).await?;
            quotes.push(to_quote(row, lines));
        }
        Ok(quotes)
    }

    pub async fn get(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        let row: QuoteRow = sqlx::query_as(&format!(
            "SELECT {QUOTE_COLS} FROM quotes WHERE id = $1 AND organization_id = $2"
        ))
        .bind(q_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, q_uuid).await?;
        Ok(to_quote(row, lines))
    }

    pub async fn create(pool: &PgPool, org_id: &str, input: CreateQuote) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let contact_uuid = input.contact_id.as_deref().map(parse_uuid).transpose()?;

        // Auto-generate quote number; fall back to UUID prefix if no sequence configured.
        let quote_number = match DocSequenceRepo::next(pool, org_id, "quote").await {
            Ok(n) => n,
            Err(DbError::NotFound) => {
                format!("QUO-{}", &Uuid::new_v4().to_string()[..8].to_uppercase())
            }
            Err(e) => return Err(e),
        };

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO quotes \
             (id, organization_id, contact_id, quote_number, issue_date, expiry_date, \
              currency, notes, terms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(contact_uuid)
        .bind(&quote_number)
        .bind(input.issue_date)
        .bind(input.expiry_date)
        .bind(&input.currency)
        .bind(&input.notes)
        .bind(&input.terms)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        for (i, line) in input.lines.into_iter().enumerate() {
            let product_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
            let sort = if line.sort_order != 0 {
                line.sort_order
            } else {
                i as i32
            };
            sqlx::query(
                "INSERT INTO quote_lines \
                 (quote_id, product_id, description, quantity, unit_price, \
                  discount_pct, tax_rate, sort_order) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(id)
            .bind(product_uuid)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.discount_pct)
            .bind(line.tax_rate)
            .bind(sort)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        }

        Self::get(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        quote_id: &str,
        input: UpdateQuote,
    ) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        // Only draft quotes can be edited.
        let row: Option<(String,)> =
            sqlx::query_as("SELECT status FROM quotes WHERE id = $1 AND organization_id = $2")
                .bind(q_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        let (status,) = row.ok_or(DbError::NotFound)?;
        if status != "draft" {
            return Err(DbError::Conflict(format!(
                "quote cannot be edited from status '{status}'"
            )));
        }

        sqlx::query(
            "UPDATE quotes SET \
             contact_id   = COALESCE($3, contact_id), \
             issue_date   = COALESCE($4, issue_date), \
             expiry_date  = COALESCE($5, expiry_date), \
             notes        = COALESCE($6, notes), \
             terms        = COALESCE($7, terms), \
             updated_at   = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(q_uuid)
        .bind(org_uuid)
        .bind(input.contact_id.as_deref().map(parse_uuid).transpose()?)
        .bind(input.issue_date)
        .bind(input.expiry_date)
        .bind(&input.notes)
        .bind(&input.terms)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if let Some(lines) = input.lines {
            sqlx::query("DELETE FROM quote_lines WHERE quote_id = $1")
                .bind(q_uuid)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;

            for (i, line) in lines.into_iter().enumerate() {
                let product_uuid = line.product_id.as_deref().map(parse_uuid).transpose()?;
                let sort = if line.sort_order != 0 {
                    line.sort_order
                } else {
                    i as i32
                };
                sqlx::query(
                    "INSERT INTO quote_lines \
                     (quote_id, product_id, description, quantity, unit_price, \
                      discount_pct, tax_rate, sort_order) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(q_uuid)
                .bind(product_uuid)
                .bind(&line.description)
                .bind(line.quantity)
                .bind(line.unit_price)
                .bind(line.discount_pct)
                .bind(line.tax_rate)
                .bind(sort)
                .execute(pool)
                .await
                .map_err(map_sqlx_err)?;
            }
        }

        Self::get(pool, org_id, quote_id).await
    }

    pub async fn delete(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        let row: Option<(String,)> =
            sqlx::query_as("SELECT status FROM quotes WHERE id = $1 AND organization_id = $2")
                .bind(q_uuid)
                .bind(org_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;
        let (status,) = row.ok_or(DbError::NotFound)?;
        if status != "draft" {
            return Err(DbError::Conflict(format!(
                "quote cannot be deleted from status '{status}'"
            )));
        }

        sqlx::query("DELETE FROM quotes WHERE id = $1 AND organization_id = $2")
            .bind(q_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }

    pub async fn send(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        let rows_affected = sqlx::query(
            "UPDATE quotes SET status = 'sent', sent_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(q_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT status FROM quotes WHERE id = $1 AND organization_id = $2")
                    .bind(q_uuid)
                    .bind(org_uuid)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "quote cannot be sent from status '{s}'"
                ))),
            };
        }

        Self::get(pool, org_id, quote_id).await
    }

    pub async fn accept(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<Quote, DbError> {
        Self::transition(pool, org_id, quote_id, "sent", "accepted", "accepted_at").await
    }

    pub async fn decline(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<Quote, DbError> {
        Self::transition(pool, org_id, quote_id, "sent", "declined", "declined_at").await
    }

    pub async fn expire(pool: &PgPool, org_id: &str, quote_id: &str) -> Result<Quote, DbError> {
        Self::transition(pool, org_id, quote_id, "sent", "expired", "").await
    }

    async fn transition(
        pool: &PgPool,
        org_id: &str,
        quote_id: &str,
        from_status: &str,
        to_status: &str,
        timestamp_col: &str,
    ) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        let sql = if timestamp_col.is_empty() {
            format!(
                "UPDATE quotes SET status = '{to_status}', updated_at = NOW() \
                 WHERE id = $1 AND organization_id = $2 AND status = '{from_status}'"
            )
        } else {
            format!(
                "UPDATE quotes SET status = '{to_status}', {timestamp_col} = NOW(), updated_at = NOW() \
                 WHERE id = $1 AND organization_id = $2 AND status = '{from_status}'"
            )
        };

        let rows_affected = sqlx::query(&sql)
            .bind(q_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?
            .rows_affected();

        if rows_affected == 0 {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT status FROM quotes WHERE id = $1 AND organization_id = $2")
                    .bind(q_uuid)
                    .bind(org_uuid)
                    .fetch_optional(pool)
                    .await
                    .map_err(map_sqlx_err)?;
            return match row {
                None => Err(DbError::NotFound),
                Some((s,)) => Err(DbError::Conflict(format!(
                    "quote cannot transition from status '{s}'"
                ))),
            };
        }

        Self::get(pool, org_id, quote_id).await
    }

    /// Convert an accepted quote to an invoice. Sets status to 'invoiced'.
    pub async fn convert_to_invoice(
        pool: &PgPool,
        org_id: &str,
        quote_id: &str,
        input: ConvertQuoteToInvoice,
    ) -> Result<Quote, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let q_uuid = parse_uuid(quote_id)?;

        let quote = Self::get(pool, org_id, quote_id).await?;
        if quote.status != "accepted" {
            return Err(DbError::Conflict(format!(
                "quote must be accepted before conversion, current status: '{}'",
                quote.status
            )));
        }
        if quote.contact_id.is_none() {
            return Err(DbError::Conflict(
                "quote must have a contact to convert to invoice".into(),
            ));
        }

        let invoice_date = input
            .invoice_date
            .unwrap_or_else(|| time::OffsetDateTime::now_utc().date());
        let due_date = input.due_date.unwrap_or(invoice_date);

        let invoice_lines: Vec<CreateInvoiceLine> = quote
            .lines
            .iter()
            .map(|l| CreateInvoiceLine {
                description: l.description.clone(),
                account_id: None,
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: Some(l.tax_rate),
                discount_pct: l.discount_pct,
                product_id: l.product_id.clone(),
                variant_id: None,
            })
            .collect();

        let create_invoice = CreateInvoice {
            contact_id: quote.contact_id.clone().unwrap(),
            invoice_type: InvoiceType::Invoice,
            date: invoice_date,
            due_date,
            currency: Some(quote.currency.clone()),
            exchange_rate: None,
            notes: quote.notes.clone(),
            global_discount_pct: 0,
            lines: invoice_lines,
        };

        let invoice = InvoiceRepo::create(pool, org_id, create_invoice).await?;
        let inv_uuid = parse_uuid(&invoice.id)?;

        sqlx::query(
            "UPDATE quotes \
             SET status = 'invoiced', invoiced_at = NOW(), converted_invoice_id = $3, \
                 updated_at = NOW() \
             WHERE id = $1 AND organization_id = $2",
        )
        .bind(q_uuid)
        .bind(org_uuid)
        .bind(inv_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get(pool, org_id, quote_id).await
    }
}
