use oxidebooks_core::models::{CreatePaymentLink, PaymentLink};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct PaymentLinkRow {
    id: Uuid,
    organization_id: Uuid,
    invoice_id: Uuid,
    token: String,
    amount_due: i64,
    currency: String,
    status: String,
    expires_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

fn from_row(r: PaymentLinkRow) -> PaymentLink {
    PaymentLink {
        id: r.id.to_string(),
        organization_id: r.organization_id.to_string(),
        invoice_id: r.invoice_id.to_string(),
        token: r.token,
        amount_due: r.amount_due,
        currency: r.currency,
        status: r.status,
        expires_at: r.expires_at,
        created_at: r.created_at,
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

fn generate_token() -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let mut bytes = [0u8; 24];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub struct PaymentLinkRepo;

impl PaymentLinkRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        input: CreatePaymentLink,
    ) -> Result<PaymentLink, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let invoice_uuid = parse_uuid(&input.invoice_id)?;

        // Fetch amount_due and currency from the invoice
        let (amount_due, currency): (i64, String) = sqlx::query_as(
            "SELECT total_amount, currency FROM invoices WHERE id = $1 AND organization_id = $2",
        )
        .bind(invoice_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        let token = generate_token();
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO payment_links \
             (organization_id, invoice_id, token, amount_due, currency, expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(org_uuid)
        .bind(invoice_uuid)
        .bind(&token)
        .bind(amount_due)
        .bind(&currency)
        .bind(input.expires_at)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, org_id, &id.to_string()).await
    }

    pub async fn get_by_id(pool: &PgPool, org_id: &str, id: &str) -> Result<PaymentLink, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let row: PaymentLinkRow = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, token, amount_due, currency, \
             status, expires_at, created_at \
             FROM payment_links WHERE id = $1 AND organization_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn get_by_token(pool: &PgPool, token: &str) -> Result<PaymentLink, DbError> {
        let row: PaymentLinkRow = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, token, amount_due, currency, \
             status, expires_at, created_at \
             FROM payment_links WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(from_row(row))
    }

    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<PaymentLink>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<PaymentLinkRow> = sqlx::query_as(
            "SELECT id, organization_id, invoice_id, token, amount_due, currency, \
             status, expires_at, created_at \
             FROM payment_links WHERE organization_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    pub async fn expire(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;
        let n = sqlx::query(
            "UPDATE payment_links SET status = 'cancelled' \
             WHERE id = $1 AND organization_id = $2 AND status = 'active'",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?
        .rows_affected();
        if n == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub async fn mark_paid(pool: &PgPool, token: &str) -> Result<(), DbError> {
        sqlx::query("UPDATE payment_links SET status = 'paid' WHERE token = $1")
            .bind(token)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;
        Ok(())
    }
}
