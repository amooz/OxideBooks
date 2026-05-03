use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use time::macros::format_description;
use time::Date;

use oxidebooks_db::repos::ExchangeRateRepo;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct RateQuery {
    pub base: String,
    pub quote: String,
    /// ISO date string, e.g. "2024-01-15". Defaults to today (UTC) if omitted.
    pub date: Option<String>,
}

/// GET /api/v1/exchange-rates?base=USD&quote=EUR&date=2024-01-15
///
/// Returns the exchange rate for the given currency pair on the given date.
/// Falls back to the most recent cached rate on or before the requested date
/// (handles weekends / holidays). Fetches from the Frankfurter API if not cached.
pub async fn get_rate(
    State(state): State<AppState>,
    Query(q): Query<RateQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let base = q.base.to_uppercase();
    let quote = q.quote.to_uppercase();

    if base.len() != 3 || quote.len() != 3 {
        return Err(ApiError::BadRequest(
            "base and quote must be 3-letter ISO 4217 currency codes".into(),
        ));
    }

    let date = match q.date {
        Some(ref s) => {
            let fmt = format_description!("[year]-[month]-[day]");
            Date::parse(s, fmt).map_err(|_| {
                ApiError::BadRequest(format!("invalid date '{}'; expected YYYY-MM-DD", s))
            })?
        }
        None => {
            let now = time::OffsetDateTime::now_utc();
            now.date()
        }
    };

    // 1. Check DB cache (exact date match first).
    if let Some(cached) = ExchangeRateRepo::get(&state.db, &base, &quote, date).await? {
        return Ok(Json(rate_response(&cached, false)));
    }

    // 2. Try fetching from the upstream provider.
    let fetched = fetch_from_provider(&state.config.app.exchange_rate_url, &base, &quote, date)
        .await
        .ok();

    if let Some((fetched_rate, fetched_date)) = fetched {
        let stored = ExchangeRateRepo::upsert(
            &state.db,
            &base,
            &quote,
            fetched_date,
            &fetched_rate.to_string(),
            "frankfurter",
        )
        .await?;
        return Ok(Json(rate_response(&stored, false)));
    }

    // 3. Fall back to the most recent cached rate on or before requested date.
    if let Some(fallback) =
        ExchangeRateRepo::get_latest_on_or_before(&state.db, &base, &quote, date).await?
    {
        return Ok(Json(rate_response(&fallback, true)));
    }

    Err(ApiError::NotFound)
}

fn rate_response(
    r: &oxidebooks_db::repos::exchange_rates::ExchangeRate,
    is_fallback: bool,
) -> serde_json::Value {
    serde_json::json!({
        "data": {
            "base_currency": r.base_currency,
            "quote_currency": r.quote_currency,
            "rate": r.rate,
            "rate_date": r.rate_date.to_string(),
            "source": r.source,
            "is_fallback": is_fallback,
        }
    })
}

/// Calls the Frankfurter-compatible API and returns `(rate, actual_date)`.
/// The API may return a different date if the requested date has no data (weekend/holiday).
async fn fetch_from_provider(
    base_url: &str,
    base: &str,
    quote: &str,
    date: Date,
) -> anyhow::Result<(f64, Date)> {
    let fmt = format_description!("[year]-[month]-[day]");
    let date_str = date.format(fmt)?;
    let url = format!(
        "{}/{date_str}?from={base}&to={quote}",
        base_url.trim_end_matches('/')
    );

    let resp: serde_json::Value = reqwest::get(&url).await?.error_for_status()?.json().await?;

    let rate = resp["rates"][quote]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("missing rate in response"))?;

    // Frankfurter returns the actual date used (may differ from requested).
    let actual_date_str = resp["date"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing date in response"))?;
    let actual_date = Date::parse(actual_date_str, fmt)?;

    Ok((rate, actual_date))
}
