use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use oxidebooks_core::models::{
    CreateInvoice, CreateInvoiceLine, CreateRecurringInvoice, CreateRecurringInvoiceLine,
    RecurringInvoice, RecurringInvoiceLine, UpdateRecurringInvoice,
};

use crate::{error::DbError, repos::InvoiceRepo};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const INV_COLS: &str = "id, organization_id, contact_id, description, reference,
    currency_code, frequency, interval_count, next_due_date, end_date,
    is_active, days_due, created_at, updated_at";

const LINE_COLS: &str =
    "id, recurring_invoice_id, description, quantity, unit_price, account_id, tax_rate, sort_order";

#[derive(sqlx::FromRow)]
struct InvRow {
    id: Uuid,
    organization_id: Uuid,
    contact_id: String,
    description: String,
    reference: Option<String>,
    currency_code: String,
    frequency: String,
    interval_count: i32,
    next_due_date: Date,
    end_date: Option<Date>,
    is_active: bool,
    days_due: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    recurring_invoice_id: Uuid,
    description: String,
    quantity: i32,
    unit_price: i64,
    account_id: Option<String>,
    tax_rate: i64,
    sort_order: i32,
}

impl From<LineRow> for RecurringInvoiceLine {
    fn from(r: LineRow) -> Self {
        RecurringInvoiceLine {
            id: r.id.to_string(),
            recurring_invoice_id: r.recurring_invoice_id.to_string(),
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            account_id: r.account_id,
            tax_rate: r.tax_rate,
            sort_order: r.sort_order,
        }
    }
}

fn to_ri(r: InvRow, lines: Vec<RecurringInvoiceLine>) -> RecurringInvoice {
    RecurringInvoice {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        contact_id: r.contact_id,
        description: r.description,
        reference: r.reference,
        currency_code: r.currency_code,
        frequency: r.frequency,
        interval_count: r.interval_count,
        next_due_date: r.next_due_date,
        end_date: r.end_date,
        is_active: r.is_active,
        days_due: r.days_due,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

async fn fetch_lines(pool: &PgPool, ri_id: Uuid) -> Result<Vec<RecurringInvoiceLine>, DbError> {
    let rows = sqlx::query_as::<_, LineRow>(&format!(
        "SELECT {LINE_COLS} FROM recurring_invoice_lines
         WHERE recurring_invoice_id = $1 ORDER BY sort_order, id"
    ))
    .bind(ri_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

fn next_date(current: Date, frequency: &str, interval: i32) -> Date {
    use time::Duration;
    match frequency {
        "weekly" => current + Duration::weeks(interval as i64),
        "monthly" => {
            let months = current.month() as i64 + interval as i64 - 1;
            let years = current.year() as i64 + months / 12;
            let month_in_year = (months % 12 + 1) as u8;
            let month = time::Month::try_from(month_in_year).unwrap_or(time::Month::January);
            let days_in_month = time::util::days_in_month(month, years as i32);
            let day = current.day().min(days_in_month);
            Date::from_calendar_date(years as i32, month, day).unwrap_or(current)
        }
        "quarterly" => {
            let months = current.month() as i64 + interval as i64 * 3 - 1;
            let years = current.year() as i64 + months / 12;
            let month_in_year = (months % 12 + 1) as u8;
            let month = time::Month::try_from(month_in_year).unwrap_or(time::Month::January);
            let days_in_month = time::util::days_in_month(month, years as i32);
            let day = current.day().min(days_in_month);
            Date::from_calendar_date(years as i32, month, day).unwrap_or(current)
        }
        _ => {
            // yearly
            Date::from_calendar_date(current.year() + interval, current.month(), current.day())
                .unwrap_or(current)
        }
    }
}

pub struct RecurringInvoiceRepo;

impl RecurringInvoiceRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<RecurringInvoice>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows = if active_only {
            sqlx::query_as::<_, InvRow>(&format!(
                "SELECT {INV_COLS} FROM recurring_invoices
                 WHERE organization_id = $1 AND is_active = TRUE ORDER BY next_due_date"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, InvRow>(&format!(
                "SELECT {INV_COLS} FROM recurring_invoices
                 WHERE organization_id = $1 ORDER BY next_due_date"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await?
        };

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let lines = fetch_lines(pool, row.id).await?;
            out.push(to_ri(row, lines));
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<RecurringInvoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ri_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, InvRow>(&format!(
            "SELECT {INV_COLS} FROM recurring_invoices
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(ri_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_ri(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateRecurringInvoice,
    ) -> Result<RecurringInvoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let currency = input.currency_code.unwrap_or_else(|| "USD".to_string());

        let row = sqlx::query_as::<_, InvRow>(&format!(
            "INSERT INTO recurring_invoices
                (organization_id, contact_id, description, reference, currency_code,
                 frequency, interval_count, next_due_date, end_date, days_due)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             RETURNING {INV_COLS}"
        ))
        .bind(org_uuid)
        .bind(&input.contact_id)
        .bind(&input.description)
        .bind(&input.reference)
        .bind(&currency)
        .bind(&input.frequency)
        .bind(input.interval_count)
        .bind(input.next_due_date)
        .bind(input.end_date)
        .bind(input.days_due)
        .fetch_one(pool)
        .await
        .map_err(crate::error::map_sqlx_err)?;

        let lines = insert_lines(pool, row.id, &input.lines).await?;
        Ok(to_ri(row, lines))
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateRecurringInvoice,
    ) -> Result<RecurringInvoice, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ri_uuid = parse_uuid(id)?;

        let row = sqlx::query_as::<_, InvRow>(&format!(
            "UPDATE recurring_invoices SET
                description    = COALESCE($3, description),
                reference      = COALESCE($4, reference),
                frequency      = COALESCE($5, frequency),
                interval_count = COALESCE($6, interval_count),
                next_due_date  = COALESCE($7, next_due_date),
                end_date       = COALESCE($8, end_date),
                days_due       = COALESCE($9, days_due),
                is_active      = COALESCE($10, is_active),
                updated_at     = now()
             WHERE organization_id = $1 AND id = $2
             RETURNING {INV_COLS}"
        ))
        .bind(org_uuid)
        .bind(ri_uuid)
        .bind(input.description)
        .bind(input.reference)
        .bind(input.frequency)
        .bind(input.interval_count)
        .bind(input.next_due_date)
        .bind(input.end_date)
        .bind(input.days_due)
        .bind(input.is_active)
        .fetch_optional(pool)
        .await
        .map_err(crate::error::map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_ri(row, lines))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let ri_uuid = parse_uuid(id)?;
        let result =
            sqlx::query("DELETE FROM recurring_invoices WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(ri_uuid)
                .execute(pool)
                .await
                .map_err(crate::error::map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Generate the next invoice from this recurring template and advance next_due_date.
    pub async fn generate(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<oxidebooks_core::models::Invoice, DbError> {
        let ri = Self::get_by_id(pool, org_id, id).await?;
        if !ri.is_active {
            return Err(DbError::Conflict("recurring invoice is inactive".into()));
        }

        let invoice_date = ri.next_due_date;
        let due = {
            use time::Duration;
            invoice_date + Duration::days(ri.days_due as i64)
        };

        let lines: Vec<CreateInvoiceLine> = ri
            .lines
            .iter()
            .map(|l| CreateInvoiceLine {
                description: l.description.clone(),
                account_id: l.account_id.clone(),
                quantity: l.quantity as i64,
                unit_price: l.unit_price,
                tax_rate: Some(l.tax_rate),
                discount_pct: 0,
                product_id: None,
                variant_id: None,
            })
            .collect();

        let input = CreateInvoice {
            contact_id: ri.contact_id.clone(),
            invoice_type: oxidebooks_core::models::InvoiceType::Invoice,
            date: invoice_date,
            due_date: due,
            currency: Some(ri.currency_code.clone()),
            exchange_rate: None,
            notes: ri.reference.clone(),
            global_discount_pct: 0,
            lines,
        };

        let invoice = InvoiceRepo::create(pool, org_id, input).await?;

        // Advance next_due_date
        let new_next = next_date(ri.next_due_date, &ri.frequency, ri.interval_count);
        let org_uuid = parse_uuid(org_id)?;
        let ri_uuid = parse_uuid(id)?;
        let still_active = ri.end_date.is_none_or(|end| new_next <= end);

        sqlx::query(
            "UPDATE recurring_invoices SET next_due_date = $3, is_active = $4, updated_at = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(ri_uuid)
        .bind(new_next)
        .bind(still_active)
        .execute(pool)
        .await?;

        Ok(invoice)
    }
}

async fn insert_lines(
    pool: &PgPool,
    ri_id: Uuid,
    lines: &[CreateRecurringInvoiceLine],
) -> Result<Vec<RecurringInvoiceLine>, DbError> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let row = sqlx::query_as::<_, LineRow>(&format!(
            "INSERT INTO recurring_invoice_lines
                (recurring_invoice_id, description, quantity, unit_price, account_id, tax_rate, sort_order)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING {LINE_COLS}"
        ))
        .bind(ri_id)
        .bind(&line.description)
        .bind(line.quantity)
        .bind(line.unit_price)
        .bind(&line.account_id)
        .bind(line.tax_rate)
        .bind(line.sort_order)
        .fetch_one(pool)
        .await
        .map_err(crate::error::map_sqlx_err)?;
        out.push(row.into());
    }
    Ok(out)
}
