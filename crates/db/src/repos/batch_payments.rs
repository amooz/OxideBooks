use oxidebooks_core::models::{BatchPayment, BatchPaymentLine, CreateBatchPayment};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct BatchPaymentRow {
    id: Uuid,
    organization_id: Uuid,
    payment_date: Date,
    method: String,
    reference: Option<String>,
    total_amount: i64,
    created_by: Option<Uuid>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct LineRow {
    id: Uuid,
    batch_payment_id: Uuid,
    invoice_id: Uuid,
    amount: i64,
}

fn line_from_row(r: LineRow) -> BatchPaymentLine {
    BatchPaymentLine {
        id: r.id.to_string(),
        batch_payment_id: r.batch_payment_id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        amount: r.amount,
    }
}

pub struct BatchPaymentRepo;

impl BatchPaymentRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<BatchPayment>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<BatchPaymentRow> = sqlx::query_as(
            "SELECT id, organization_id, payment_date, method, reference, total_amount, \
             created_by, created_at FROM batch_payments WHERE organization_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut payments = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.id;
            let lines = Self::fetch_lines(pool, id).await?;
            payments.push(BatchPayment {
                id: row.id.to_string(),
                organization_id: row.organization_id.to_string(),
                payment_date: row.payment_date,
                method: row.method,
                reference: row.reference,
                total_amount: row.total_amount,
                created_by: row.created_by.map(|u| u.to_string()),
                created_at: row.created_at,
                lines,
            });
        }
        Ok(payments)
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<BatchPayment, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: BatchPaymentRow = sqlx::query_as(
            "SELECT id, organization_id, payment_date, method, reference, total_amount, \
             created_by, created_at FROM batch_payments WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let lines = Self::fetch_lines(pool, row.id).await?;
        Ok(BatchPayment {
            id: row.id.to_string(),
            organization_id: row.organization_id.to_string(),
            payment_date: row.payment_date,
            method: row.method,
            reference: row.reference,
            total_amount: row.total_amount,
            created_by: row.created_by.map(|u| u.to_string()),
            created_at: row.created_at,
            lines,
        })
    }

    /// Create a batch payment run, returning the batch record plus lists of
    /// succeeded and failed invoice IDs.
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        user_id: &str,
        input: CreateBatchPayment,
        payment_date: Date,
    ) -> Result<(BatchPayment, Vec<String>, Vec<(String, String)>), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let user_uuid = parse_uuid(user_id)?;

        let batch_id: Uuid = sqlx::query_scalar(
            "INSERT INTO batch_payments (organization_id, payment_date, method, reference, created_by) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(org_uuid)
        .bind(payment_date)
        .bind(&input.method)
        .bind(&input.reference)
        .bind(user_uuid)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let mut succeeded = Vec::new();
        let mut failed: Vec<(String, String)> = Vec::new();
        let mut total: i64 = 0;

        for inv_id in &input.invoice_ids {
            match process_invoice(
                pool,
                org_uuid,
                batch_id,
                inv_id,
                payment_date,
                &input.method,
            )
            .await
            {
                Ok(amount) => {
                    total += amount;
                    succeeded.push(inv_id.clone());
                }
                Err(e) => failed.push((inv_id.clone(), e.to_string())),
            }
        }

        // Update total on batch
        sqlx::query("UPDATE batch_payments SET total_amount = $1 WHERE id = $2")
            .bind(total)
            .bind(batch_id)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        let batch = Self::get_by_id(pool, org_id, &batch_id.to_string()).await?;
        Ok((batch, succeeded, failed))
    }

    async fn fetch_lines(pool: &PgPool, batch_id: Uuid) -> Result<Vec<BatchPaymentLine>, DbError> {
        let rows: Vec<LineRow> = sqlx::query_as(
            "SELECT id, batch_payment_id, invoice_id, amount FROM batch_payment_lines \
             WHERE batch_payment_id = $1",
        )
        .bind(batch_id)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(line_from_row).collect())
    }
}

async fn process_invoice(
    pool: &PgPool,
    org_uuid: Uuid,
    batch_id: Uuid,
    inv_id: &str,
    payment_date: Date,
    method: &str,
) -> Result<i64, DbError> {
    let inv_uuid = parse_uuid(inv_id)?;

    #[derive(sqlx::FromRow)]
    struct InvRow {
        status: String,
        total_cents: i64,
    }

    let inv: InvRow = sqlx::query_as(
        "SELECT i.status, \
         COALESCE(SUM(il.quantity * il.unit_price / 100 + il.quantity * il.unit_price / 100 * il.tax_rate / 10000), 0)::BIGINT AS total_cents \
         FROM invoices i LEFT JOIN invoice_lines il ON i.id = il.invoice_id \
         WHERE i.id = $1 AND i.organization_id = $2 \
         GROUP BY i.status",
    )
    .bind(inv_uuid)
    .bind(org_uuid)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?
    .ok_or(DbError::NotFound)?;

    if inv.status != "sent" && inv.status != "overdue" {
        return Err(DbError::Conflict(format!(
            "invoice {} is not payable (status: {})",
            inv_id, inv.status
        )));
    }

    let amount = inv.total_cents;

    sqlx::query(
        "INSERT INTO payments (organization_id, invoice_id, amount, payment_date, method) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(org_uuid)
    .bind(inv_uuid)
    .bind(amount)
    .bind(payment_date)
    .bind(method)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    sqlx::query(
        "UPDATE invoices SET status = 'paid', updated_at = NOW() \
         WHERE id = $1 AND organization_id = $2",
    )
    .bind(inv_uuid)
    .bind(org_uuid)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    sqlx::query(
        "INSERT INTO batch_payment_lines (batch_payment_id, invoice_id, amount) \
         VALUES ($1,$2,$3)",
    )
    .bind(batch_id)
    .bind(inv_uuid)
    .bind(amount)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(amount)
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
