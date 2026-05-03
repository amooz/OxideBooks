use oxidebooks_core::models::{CreatePayment, Payment};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PaymentRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    amount: i64,
    payment_date: Date,
    method: String,
    reference: Option<String>,
    notes: Option<String>,
    created_at: OffsetDateTime,
}

impl From<PaymentRow> for Payment {
    fn from(r: PaymentRow) -> Self {
        Payment {
            id: r.id.to_string(),
            organization_id: r.organization_id.to_string(),
            invoice_id: r.invoice_id.to_string(),
            amount: r.amount,
            payment_date: r.payment_date,
            method: r.method,
            reference: r.reference,
            notes: r.notes,
            created_at: r.created_at,
        }
    }
}

pub struct PaymentRepo;

impl PaymentRepo {
    /// Record a payment against an invoice and auto-update its status.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
        input: CreatePayment,
    ) -> Result<Payment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;
        let id = Uuid::new_v4();

        // Verify the invoice belongs to this org.
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM invoices WHERE organization_id = $1 AND id = $2")
                .bind(org_uuid)
                .bind(inv_uuid)
                .fetch_optional(pool)
                .await
                .map_err(map_sqlx_err)?;

        if exists.is_none() {
            return Err(DbError::NotFound);
        }

        sqlx::query(
            "INSERT INTO payments \
             (id, organization_id, invoice_id, amount, payment_date, method, reference, notes) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(inv_uuid)
        .bind(input.amount)
        .bind(input.payment_date)
        .bind(&input.method)
        .bind(&input.reference)
        .bind(&input.notes)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        // Compute total paid and invoice total, update status.
        Self::sync_invoice_status(pool, org_uuid, inv_uuid).await?;

        let row: PaymentRow = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, amount, payment_date, method, \
             reference, notes, created_at FROM payments WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    pub async fn list_by_invoice(
        pool: &PgPool,
        org_id: &str,
        invoice_id: &str,
    ) -> Result<Vec<Payment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let inv_uuid = parse_uuid(invoice_id)?;

        let rows: Vec<PaymentRow> = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, amount, payment_date, method, \
             reference, notes, created_at \
             FROM payments WHERE organization_id = $1 AND invoice_id = $2 \
             ORDER BY payment_date ASC, created_at ASC",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Payment::from).collect())
    }

    /// Recompute invoice status based on total payments vs. invoice line total.
    pub(crate) async fn sync_invoice_status(
        pool: &PgPool,
        org_uuid: Uuid,
        inv_uuid: Uuid,
    ) -> Result<(), DbError> {
        // Invoice total = sum of quantity * unit_price * (1 + tax_rate/1_000_000)
        // tax_rate is stored as basis points of a basis point, i.e. 1% = 10000.
        // Simplified: total_minor_units = sum(quantity * unit_price + quantity * unit_price * tax_rate / 1_000_000)
        let invoice_total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(quantity * unit_price + quantity * unit_price * tax_rate / 1000000), 0)::BIGINT \
             FROM invoice_lines WHERE invoice_id = $1",
        )
        .bind(inv_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let paid_total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM payments \
             WHERE organization_id = $1 AND invoice_id = $2",
        )
        .bind(org_uuid)
        .bind(inv_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let new_status = if paid_total.0 >= invoice_total.0 {
            "paid"
        } else if paid_total.0 > 0 {
            "partial"
        } else {
            return Ok(());
        };

        sqlx::query(
            "UPDATE invoices SET status = $1, updated_at = NOW() \
             WHERE organization_id = $2 AND id = $3 AND status NOT IN ('voided', 'paid')",
        )
        .bind(new_status)
        .bind(org_uuid)
        .bind(inv_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
