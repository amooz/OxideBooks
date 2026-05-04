use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use oxidebooks_core::models::{
    CreateBillLine, CreateRecurringBill, CreateRecurringBillLine, CreateVendorBill, RecurringBill,
    RecurringBillLine, UpdateRecurringBill,
};

use crate::{error::DbError, repos::BillRepo};

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

const BILL_COLS: &str = "id, organization_id, contact_id, description, reference,
    currency_code, frequency, interval_count, next_due_date, end_date,
    is_active, days_due, created_at, updated_at";

const LINE_COLS: &str =
    "id, recurring_bill_id, description, quantity, unit_price, account_id, tax_rate, sort_order";

#[derive(sqlx::FromRow)]
struct BillRow {
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
    recurring_bill_id: Uuid,
    description: String,
    quantity: i32,
    unit_price: i64,
    account_id: Option<String>,
    tax_rate: i64,
    sort_order: i32,
}

impl From<LineRow> for RecurringBillLine {
    fn from(r: LineRow) -> Self {
        RecurringBillLine {
            id: r.id.to_string(),
            recurring_bill_id: r.recurring_bill_id.to_string(),
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            account_id: r.account_id,
            tax_rate: r.tax_rate,
            sort_order: r.sort_order,
        }
    }
}

fn to_rb(r: BillRow, lines: Vec<RecurringBillLine>) -> RecurringBill {
    RecurringBill {
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

async fn fetch_lines(pool: &PgPool, rb_id: Uuid) -> Result<Vec<RecurringBillLine>, DbError> {
    let rows = sqlx::query_as::<_, LineRow>(&format!(
        "SELECT {LINE_COLS} FROM recurring_bill_lines
         WHERE recurring_bill_id = $1 ORDER BY sort_order, id"
    ))
    .bind(rb_id)
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

pub struct RecurringBillRepo;

impl RecurringBillRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<RecurringBill>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows = if active_only {
            sqlx::query_as::<_, BillRow>(&format!(
                "SELECT {BILL_COLS} FROM recurring_bills
                 WHERE organization_id = $1 AND is_active = TRUE ORDER BY next_due_date"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, BillRow>(&format!(
                "SELECT {BILL_COLS} FROM recurring_bills
                 WHERE organization_id = $1 ORDER BY next_due_date"
            ))
            .bind(org_uuid)
            .fetch_all(pool)
            .await?
        };

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let lines = fetch_lines(pool, row.id).await?;
            out.push(to_rb(row, lines));
        }
        Ok(out)
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<RecurringBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rb_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, BillRow>(&format!(
            "SELECT {BILL_COLS} FROM recurring_bills
             WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org_uuid)
        .bind(rb_uuid)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_rb(row, lines))
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateRecurringBill,
    ) -> Result<RecurringBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let currency = input.currency_code.unwrap_or_else(|| "USD".to_string());

        let row = sqlx::query_as::<_, BillRow>(&format!(
            "INSERT INTO recurring_bills
                (organization_id, contact_id, description, reference, currency_code,
                 frequency, interval_count, next_due_date, end_date, days_due)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             RETURNING {BILL_COLS}"
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
        .await?;

        let rb_id = row.id;
        Self::insert_lines(pool, rb_id, &input.lines).await?;
        let lines = fetch_lines(pool, rb_id).await?;
        Ok(to_rb(row, lines))
    }

    async fn insert_lines(
        pool: &PgPool,
        rb_id: Uuid,
        lines: &[CreateRecurringBillLine],
    ) -> Result<(), DbError> {
        for (i, line) in lines.iter().enumerate() {
            sqlx::query(
                "INSERT INTO recurring_bill_lines
                    (recurring_bill_id, description, quantity, unit_price, account_id,
                     tax_rate, sort_order)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(rb_id)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(&line.account_id)
            .bind(line.tax_rate)
            .bind(if line.sort_order != 0 {
                line.sort_order
            } else {
                i as i32
            })
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateRecurringBill,
    ) -> Result<RecurringBill, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rb_uuid = parse_uuid(id)?;
        let row = sqlx::query_as::<_, BillRow>(&format!(
            "UPDATE recurring_bills SET
                description   = COALESCE($3, description),
                reference     = COALESCE($4, reference),
                frequency     = COALESCE($5, frequency),
                interval_count= COALESCE($6, interval_count),
                next_due_date = COALESCE($7, next_due_date),
                end_date      = COALESCE($8, end_date),
                days_due      = COALESCE($9, days_due),
                is_active     = COALESCE($10, is_active),
                updated_at    = now()
             WHERE organization_id = $1 AND id = $2
             RETURNING {BILL_COLS}"
        ))
        .bind(org_uuid)
        .bind(rb_uuid)
        .bind(&input.description)
        .bind(&input.reference)
        .bind(&input.frequency)
        .bind(input.interval_count)
        .bind(input.next_due_date)
        .bind(input.end_date)
        .bind(input.days_due)
        .bind(input.is_active)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)?;

        let lines = fetch_lines(pool, row.id).await?;
        Ok(to_rb(row, lines))
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rb_uuid = parse_uuid(id)?;
        let affected =
            sqlx::query("DELETE FROM recurring_bills WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(rb_uuid)
                .execute(pool)
                .await?
                .rows_affected();
        if affected == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Generate a draft vendor bill from this recurring template and advance next_due_date.
    pub async fn generate(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<oxidebooks_core::models::VendorBill, DbError> {
        let rb = Self::get_by_id(pool, org_id, id).await?;
        if !rb.is_active {
            return Err(DbError::Conflict("recurring bill is inactive".into()));
        }

        let bill_date = rb.next_due_date;
        let due =
            time::Date::from_calendar_date(bill_date.year(), bill_date.month(), bill_date.day())
                .ok()
                .map(|d| {
                    use time::Duration;
                    d + Duration::days(rb.days_due as i64)
                });

        let lines: Vec<CreateBillLine> = rb
            .lines
            .iter()
            .map(|l| CreateBillLine {
                account_id: l.account_id.clone(),
                description: Some(l.description.clone()),
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.tax_rate,
                variant_id: None,
            })
            .collect();

        let input = CreateVendorBill {
            contact_id: Some(rb.contact_id.clone()),
            bill_date,
            due_date: due,
            reference: rb.reference.clone(),
            description: rb.description.clone(),
            currency_code: rb.currency_code.clone(),
            exchange_rate: rust_decimal::Decimal::ONE,
            lines,
            purchase_order_id: None,
        };

        let bill = BillRepo::create(pool, org_id, input).await?;

        // Advance next_due_date
        let new_next = next_date(rb.next_due_date, &rb.frequency, rb.interval_count);
        let org_uuid = parse_uuid(org_id)?;
        let rb_uuid = parse_uuid(id)?;

        // If past end_date, deactivate
        let still_active = rb.end_date.is_none_or(|end| new_next <= end);
        sqlx::query(
            "UPDATE recurring_bills SET next_due_date = $3, is_active = $4, updated_at = now()
             WHERE organization_id = $1 AND id = $2",
        )
        .bind(org_uuid)
        .bind(rb_uuid)
        .bind(new_next)
        .bind(still_active)
        .execute(pool)
        .await?;

        Ok(bill)
    }
}
