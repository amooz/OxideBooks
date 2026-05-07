use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use oxidebooks_db::repos::TaxFilingRepo;
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
pub struct YearQuery {
    pub year: i32,
}

#[derive(Deserialize)]
pub struct YearQuarterQuery {
    pub year: i32,
    pub quarter: i32,
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    pub from: String,
    pub to: String,
}

pub async fn list_tax_filings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let filings = TaxFilingRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": filings })))
}

pub async fn get_tax_filing(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let filing = TaxFilingRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn submit_tax_filing(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let filing = TaxFilingRepo::submit(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn generate_1099s(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let filing = TaxFilingRepo::generate_1099s(&state.db, &claims.org, q.year).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn generate_941(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuarterQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let filing = TaxFilingRepo::generate_941(&state.db, &claims.org, q.year, q.quarter).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn generate_t4s(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let filing = TaxFilingRepo::generate_t4s(&state.db, &claims.org, q.year).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn generate_t4a(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let filing = TaxFilingRepo::generate_t4a(&state.db, &claims.org, q.year).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn generate_hst_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    let filing = TaxFilingRepo::generate_hst_return(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": filing })))
}

pub async fn t4_summary_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let summary = TaxFilingRepo::t4_summary(&state.db, &claims.org, q.year).await?;
    Ok(Json(serde_json::json!({ "data": summary })))
}

pub async fn t4a_summary_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<YearQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let summary = TaxFilingRepo::t4a_summary(&state.db, &claims.org, q.year).await?;
    Ok(Json(serde_json::json!({ "data": summary })))
}

pub async fn hst_gst_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    let report = TaxFilingRepo::hst_gst_return(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
