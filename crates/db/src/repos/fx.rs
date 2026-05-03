use oxidebooks_core::models::{FxSummaryRow, RealizedFxEntry};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(sqlx::FromRow)]
struct FxRow {
    id: Uuid,
    organization_id: Uuid,
    payment_id: Uuid,
    invoice_currency: String,
    payment_currency: String,
    invoice_amount: i64,
    payment_amount: i64,
    fx_rate: f64,
    gain_loss: i64,
    journal_entry_id: Option<Uuid>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    period: String,
    total_gains: i64,
    total_losses: i64,
    net: i64,
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}

pub struct FxRepo;

impl FxRepo {
    /// Record a realized FX gain/loss when payment currency differs from invoice currency.
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        pool: &PgPool,
        org_id: &str,
        payment_id: &str,
        invoice_currency: &str,
        payment_currency: &str,
        invoice_amount: i64,
        payment_amount: i64,
        fx_rate: f64,
    ) -> Result<RealizedFxEntry, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let payment_uuid = parse_uuid(payment_id)?;

        // gain_loss: positive = gain, negative = loss (from perspective of invoice currency)
        let gain_loss = payment_amount - invoice_amount;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO realized_fx_entries \
             (organization_id, payment_id, invoice_currency, payment_currency, \
              invoice_amount, payment_amount, fx_rate, gain_loss) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (payment_id) DO UPDATE SET \
             fx_rate = EXCLUDED.fx_rate, gain_loss = EXCLUDED.gain_loss \
             RETURNING id",
        )
        .bind(org_uuid)
        .bind(payment_uuid)
        .bind(invoice_currency)
        .bind(payment_currency)
        .bind(invoice_amount)
        .bind(payment_amount)
        .bind(fx_rate)
        .bind(gain_loss)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Self::get_by_id(pool, &id.to_string()).await
    }

    pub async fn get_by_id(pool: &PgPool, id: &str) -> Result<RealizedFxEntry, DbError> {
        let id_uuid = parse_uuid(id)?;
        let row: FxRow = sqlx::query_as(
            "SELECT id, organization_id, payment_id, invoice_currency, payment_currency, \
             invoice_amount, payment_amount, fx_rate, gain_loss, journal_entry_id, created_at \
             FROM realized_fx_entries WHERE id = $1",
        )
        .bind(id_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;
        Ok(RealizedFxEntry {
            id: row.id.to_string(),
            organization_id: row.organization_id.to_string(),
            payment_id: row.payment_id.to_string(),
            invoice_currency: row.invoice_currency,
            payment_currency: row.payment_currency,
            invoice_amount: row.invoice_amount,
            payment_amount: row.payment_amount,
            fx_rate: row.fx_rate,
            gain_loss: row.gain_loss,
            journal_entry_id: row.journal_entry_id.map(|u| u.to_string()),
            created_at: row.created_at,
        })
    }

    pub async fn fx_summary(
        pool: &PgPool,
        org_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<FxSummaryRow>, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT \
               TO_CHAR(DATE_TRUNC('month', created_at), 'YYYY-MM') AS period, \
               COALESCE(SUM(CASE WHEN gain_loss > 0 THEN gain_loss ELSE 0 END), 0) AS total_gains, \
               COALESCE(SUM(CASE WHEN gain_loss < 0 THEN ABS(gain_loss) ELSE 0 END), 0) AS total_losses, \
               COALESCE(SUM(gain_loss), 0) AS net \
             FROM realized_fx_entries \
             WHERE organization_id = $1 \
               AND ($2::text IS NULL OR created_at::date >= $2::date) \
               AND ($3::text IS NULL OR created_at::date <= $3::date) \
             GROUP BY DATE_TRUNC('month', created_at) \
             ORDER BY DATE_TRUNC('month', created_at)",
        )
        .bind(org_uuid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(rows
            .into_iter()
            .map(|r| FxSummaryRow {
                period: r.period,
                total_gains: r.total_gains,
                total_losses: r.total_losses,
                net: r.net,
            })
            .collect())
    }
}
