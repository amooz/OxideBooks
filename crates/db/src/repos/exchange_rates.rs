use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::{map_sqlx_err, DbError};

#[derive(Debug, Clone)]
pub struct ExchangeRate {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub rate: f64,
    pub rate_date: Date,
    pub source: String,
    pub created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct RateRow {
    id: Uuid,
    base_currency: String,
    quote_currency: String,
    rate: f64,
    rate_date: Date,
    source: String,
    created_at: OffsetDateTime,
}

pub struct ExchangeRateRepo;

impl ExchangeRateRepo {
    /// Look up a cached rate for (base, quote, date).  Returns None if not cached.
    pub async fn get(
        pool: &PgPool,
        base: &str,
        quote: &str,
        date: Date,
    ) -> Result<Option<ExchangeRate>, DbError> {
        let row: Option<RateRow> = sqlx::query_as(
            "SELECT id, base_currency, quote_currency, rate, rate_date, source, created_at \
             FROM exchange_rates \
             WHERE base_currency = $1 AND quote_currency = $2 AND rate_date = $3",
        )
        .bind(base)
        .bind(quote)
        .bind(date)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|r| r.into()))
    }

    /// Upsert a rate (insert or replace if source/rate changed).
    pub async fn upsert(
        pool: &PgPool,
        base: &str,
        quote: &str,
        date: Date,
        rate: &str,
        source: &str,
    ) -> Result<ExchangeRate, DbError> {
        let id = Uuid::new_v4();

        let row: RateRow = sqlx::query_as(
            "INSERT INTO exchange_rates \
             (id, base_currency, quote_currency, rate, rate_date, source) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (base_currency, quote_currency, rate_date) \
             DO UPDATE SET rate = EXCLUDED.rate, source = EXCLUDED.source, \
                           created_at = NOW() \
             RETURNING id, base_currency, quote_currency, rate, rate_date, source, created_at",
        )
        .bind(id)
        .bind(base)
        .bind(quote)
        .bind(rate.parse::<f64>().unwrap_or_default())
        .bind(date)
        .bind(source)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.into())
    }

    /// Fall back to the most recent available rate on or before `date`
    /// (handles weekends / holidays when ECB does not publish).
    pub async fn get_latest_on_or_before(
        pool: &PgPool,
        base: &str,
        quote: &str,
        date: Date,
    ) -> Result<Option<ExchangeRate>, DbError> {
        let row: Option<RateRow> = sqlx::query_as(
            "SELECT id, base_currency, quote_currency, rate, rate_date, source, created_at \
             FROM exchange_rates \
             WHERE base_currency = $1 AND quote_currency = $2 AND rate_date <= $3 \
             ORDER BY rate_date DESC \
             LIMIT 1",
        )
        .bind(base)
        .bind(quote)
        .bind(date)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(row.map(|r| r.into()))
    }
}

impl From<RateRow> for ExchangeRate {
    fn from(r: RateRow) -> Self {
        ExchangeRate {
            id: r.id.to_string(),
            base_currency: r.base_currency,
            quote_currency: r.quote_currency,
            rate: r.rate,
            rate_date: r.rate_date,
            source: r.source,
            created_at: r.created_at,
        }
    }
}
