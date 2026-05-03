use axum::{
    extract::{Extension, Query, State},
    Json,
};
use oxidebooks_db::repos::ReportRepo;
use serde::Deserialize;
use time::macros::format_description;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn parse_date(s: &str) -> Result<Date, ApiError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt)
        .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
pub struct AsOfQuery {
    pub as_of: String,
}

/// GET /api/v1/reports/trial-balance
///
/// Returns the trial balance for the authenticated organization. Only `posted`
/// journal entries are included.
pub async fn trial_balance(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let tb = ReportRepo::trial_balance(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({
        "data": {
            "accounts": tb.accounts,
            "total_debits": tb.total_debits,
            "total_credits": tb.total_credits,
            "is_balanced": tb.is_balanced(),
        }
    })))
}

/// GET /api/v1/reports/profit-loss?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn profit_loss(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::profit_loss(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/balance-sheet?as_of=YYYY-MM-DD
pub async fn balance_sheet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::balance_sheet(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct AgingQuery {
    pub as_of: String,
    #[serde(rename = "type")]
    pub aging_type: Option<String>,
}

/// GET /api/v1/reports/aging?type=receivable|payable&as_of=YYYY-MM-DD
pub async fn aging(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AgingQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let aging_type = q.aging_type.as_deref().unwrap_or("receivable");
    if aging_type != "receivable" && aging_type != "payable" {
        return Err(ApiError::BadRequest(
            "type must be 'receivable' or 'payable'".into(),
        ));
    }
    let report = ReportRepo::aging(&state.db, &claims.org, aging_type, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/tax-summary?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn tax_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::tax_summary(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/cash-flow?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn cash_flow(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::cash_flow(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/dashboard
pub async fn dashboard(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let kpis = ReportRepo::dashboard(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": kpis })))
}
