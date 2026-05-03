use oxidebooks_core::models::{
    ConvertSoToInvoice, CreateInvoice, CreateInvoiceLine, CreateSalesOrder, InvoiceType,
    SalesOrder, SoLine, UpdateSalesOrder,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};
use crate::repos::InvoiceRepo;

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

#[derive(sqlx::FromRow)]
struct SoRow {
    id: Uuid,
    organization_id: Uuid,
    order_number: String,
    contact_id: Uuid,
    status: String,
    order_date: Date,
    expected_ship: Option<Date>,
    currency: String,
    notes: Option<String>,
    total_amount: i64,
    invoiced_amount: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct SoLineRow {
    id: Uuid,
    so_id: Uuid,
    product_id: Option<Uuid>,
    description: String,
    quantity: i64,
    unit_price: i64,
    tax_rate: i64,
    discount_pct: i64,
    quantity_invoiced: i64,
    sort_order: i32,
}

fn line_from_row(r: SoLineRow) -> SoLine {
    let gross = r.quantity * r.unit_price / 100;
    let discount = gross * r.discount_pct / 10_000;
    let line_total = gross - discount;
    SoLine {
        id: r.id.to_string(),
        so_id: r.so_id.to_string(),
        product_id: r.product_id.map(|u| u.to_string()),
        description: r.description,
        quantity: r.quantity,
        unit_price: r.unit_price,
        tax_rate: r.tax_rate,
        discount_pct: r.discount_pct,
        quantity_invoiced: r.quantity_invoiced,
        sort_order: r.sort_order,
        line_total,
    }
}

async fn fetch_lines(pool: &PgPool, so_id: Uuid) -> Result<Vec<SoLine>, DbError> {
    let rows: Vec<SoLineRow> = sqlx::query_as(
        "SELECT id, so_id, product_id, description, quantity, unit_price, tax_rate,
                discount_pct, quantity_invoiced, sort_order
         FROM sales_order_lines WHERE so_id = $1 ORDER BY sort_order, id",
    )
    .bind(so_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;
    Ok(rows.into_iter().map(line_from_row).collect())
}

async fn so_from_row(pool: &PgPool, r: SoRow) -> Result<SalesOrder, DbError> {
    let lines = fetch_lines(pool, r.id).await?;
    let remaining = r.total_amount - r.invoiced_amount;
    Ok(SalesOrder {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        order_number: r.order_number,
        contact_id: r.contact_id.to_string(),
        status: r.status,
        order_date: r.order_date,
        expected_ship: r.expected_ship,
        currency: r.currency,
        notes: r.notes,
        total_amount: r.total_amount,
        invoiced_amount: r.invoiced_amount,
        remaining_amount: remaining,
        lines,
        created_at: r.created_at,
        updated_at: r.updated_at,
    })
}

const SO_COLS: &str = "id, organization_id, order_number, contact_id, status, order_date,
     expected_ship, currency, notes, total_amount, invoiced_amount, created_at, updated_at";

async fn generate_so_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
) -> Result<String, DbError> {
    let next_val: i64 = sqlx::query_scalar(
        "INSERT INTO so_counters (organization_id, next_val)
         VALUES ($1, 2)
         ON CONFLICT (organization_id)
         DO UPDATE SET next_val = so_counters.next_val + 1
         RETURNING next_val - 1",
    )
    .bind(org_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    Ok(format!("SO-{:05}", next_val))
}

pub struct SalesOrderRepo;

impl SalesOrderRepo {
    pub async fn list(
        pool: &PgPool,
        org_id: &str,
        status: Option<&str>,
        contact_id: Option<&str>,
    ) -> Result<Vec<SalesOrder>, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = contact_id.map(parse_uuid).transpose()?;
        let rows: Vec<SoRow> = sqlx::query_as(&format!(
            "SELECT {SO_COLS} FROM sales_orders
             WHERE organization_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
               AND ($3::UUID IS NULL OR contact_id = $3)
             ORDER BY order_date DESC, created_at DESC"
        ))
        .bind(org)
        .bind(status)
        .bind(contact)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(so_from_row(pool, r).await?);
        }
        Ok(out)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<SalesOrder, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(id)?;
        let row: SoRow = sqlx::query_as(&format!(
            "SELECT {SO_COLS} FROM sales_orders WHERE organization_id = $1 AND id = $2"
        ))
        .bind(org)
        .bind(so_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        so_from_row(pool, row).await
    }

    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreateSalesOrder,
    ) -> Result<SalesOrder, DbError> {
        let org = parse_uuid(org_id)?;
        let contact = parse_uuid(&input.contact_id)?;
        let id = Uuid::new_v4();

        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
        let order_number = generate_so_number(&mut tx, org).await?;

        sqlx::query(
            "INSERT INTO sales_orders
                (id, organization_id, order_number, contact_id, order_date, expected_ship,
                 currency, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(org)
        .bind(&order_number)
        .bind(contact)
        .bind(input.order_date)
        .bind(input.expected_ship)
        .bind(&input.currency)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let mut total: i64 = 0;
        for (i, line) in input.lines.iter().enumerate() {
            let product = line.product_id.as_deref().map(parse_uuid).transpose()?;
            let gross = line.quantity * line.unit_price / 100;
            let discount = gross * line.discount_pct / 10_000;
            total += gross - discount;
            sqlx::query(
                "INSERT INTO sales_order_lines
                    (so_id, product_id, description, quantity, unit_price, tax_rate,
                     discount_pct, sort_order)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(id)
            .bind(product)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(line.discount_pct)
            .bind(line.sort_order.max(i as i32))
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        sqlx::query("UPDATE sales_orders SET total_amount = $2 WHERE id = $1")
            .bind(id)
            .bind(total)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn update(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: UpdateSalesOrder,
    ) -> Result<SalesOrder, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(id)?;
        let row: Option<SoRow> = sqlx::query_as(&format!(
            "UPDATE sales_orders
             SET expected_ship = COALESCE($3, expected_ship),
                 currency      = COALESCE($4, currency),
                 notes         = COALESCE($5, notes),
                 updated_at    = now()
             WHERE organization_id = $1 AND id = $2
               AND status IN ('draft','confirmed')
             RETURNING {SO_COLS}"
        ))
        .bind(org)
        .bind(so_id)
        .bind(input.expected_ship)
        .bind(input.currency)
        .bind(input.notes)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        match row {
            Some(r) => so_from_row(pool, r).await,
            None => Err(DbError::NotFound),
        }
    }

    pub async fn confirm(pool: &PgPool, org_id: &str, id: &str) -> Result<SalesOrder, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE sales_orders SET status = 'confirmed', updated_at = now()
             WHERE organization_id = $1 AND id = $2 AND status = 'draft'",
        )
        .bind(org)
        .bind(so_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "sales order must be in draft status to confirm".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    pub async fn cancel(pool: &PgPool, org_id: &str, id: &str) -> Result<SalesOrder, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE sales_orders SET status = 'cancelled', updated_at = now()
             WHERE organization_id = $1 AND id = $2
               AND status IN ('draft','confirmed','partially_invoiced')",
        )
        .bind(org)
        .bind(so_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::Conflict(
                "sales order cannot be cancelled in its current state".into(),
            ));
        }
        Self::get_by_id(pool, org_id, id).await
    }

    /// Convert confirmed (or partially_invoiced) SO lines to a draft invoice.
    /// If `line_ids` is empty, all uninvoiced lines are included.
    pub async fn convert_to_invoice(
        pool: &PgPool,
        org_id: &str,
        id: &str,
        input: ConvertSoToInvoice,
    ) -> Result<oxidebooks_core::models::Invoice, DbError> {
        let org = parse_uuid(org_id)?;
        let so_id = parse_uuid(id)?;

        let so_row: SoRow = sqlx::query_as(&format!(
            "SELECT {SO_COLS} FROM sales_orders
             WHERE organization_id = $1 AND id = $2
               AND status IN ('confirmed','partially_invoiced')
             FOR UPDATE"
        ))
        .bind(org)
        .bind(so_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or_else(|| {
            DbError::Conflict(
                "sales order must be confirmed or partially invoiced to convert".into(),
            )
        })?;

        let all_lines = fetch_lines(pool, so_id).await?;
        let lines_to_invoice: Vec<&SoLine> = if input.line_ids.is_empty() {
            all_lines
                .iter()
                .filter(|l| l.quantity_invoiced < l.quantity)
                .collect()
        } else {
            all_lines
                .iter()
                .filter(|l| input.line_ids.contains(&l.id))
                .collect()
        };

        if lines_to_invoice.is_empty() {
            return Err(DbError::Conflict("no uninvoiced lines to convert".into()));
        }

        let today = time::OffsetDateTime::now_utc().date();
        let invoice_lines: Vec<CreateInvoiceLine> = lines_to_invoice
            .iter()
            .map(|l| CreateInvoiceLine {
                description: l.description.clone(),
                account_id: None,
                quantity: l.quantity - l.quantity_invoiced,
                unit_price: l.unit_price,
                tax_rate: Some(l.tax_rate),
                discount_pct: l.discount_pct,
                product_id: l.product_id.clone(),
            })
            .collect();

        let inv_input = CreateInvoice {
            contact_id: so_row.contact_id.to_string(),
            invoice_type: InvoiceType::Invoice,
            date: today,
            due_date: today,
            currency: Some(so_row.currency.clone()),
            exchange_rate: None,
            notes: so_row.notes.clone(),
            global_discount_pct: 0,
            lines: invoice_lines,
        };

        let invoice = InvoiceRepo::create(pool, org_id, inv_input).await?;

        // Mark lines as fully invoiced and update SO totals
        let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
        for l in &lines_to_invoice {
            let line_uuid = parse_uuid(&l.id)?;
            sqlx::query(
                "UPDATE sales_order_lines
                 SET quantity_invoiced = quantity
                 WHERE id = $1",
            )
            .bind(line_uuid)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // Recalculate invoiced_amount and set new status
        let invoiced_amount: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(
                (quantity_invoiced * unit_price / 100) -
                (quantity_invoiced * unit_price / 100 * discount_pct / 10000)
             ), 0)
             FROM sales_order_lines WHERE so_id = $1",
        )
        .bind(so_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let remaining_uninvoiced: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sales_order_lines
             WHERE so_id = $1 AND quantity_invoiced < quantity",
        )
        .bind(so_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let new_status = if remaining_uninvoiced == 0 {
            "fully_invoiced"
        } else {
            "partially_invoiced"
        };

        // Link the invoice back to this SO
        let inv_uuid = parse_uuid(&invoice.id)?;
        sqlx::query("UPDATE invoices SET sales_order_id = $2 WHERE id = $1")
            .bind(inv_uuid)
            .bind(so_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;

        sqlx::query(
            "UPDATE sales_orders
             SET invoiced_amount = $2, status = $3, updated_at = now()
             WHERE id = $1",
        )
        .bind(so_id)
        .bind(invoiced_amount)
        .bind(new_status)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(invoice)
    }
}
