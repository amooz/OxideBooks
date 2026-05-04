use axum::{
    extract::{Extension, Query, State},
    Json,
};
use oxidebooks_db::repos::ReportRepo;
use serde::Deserialize;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

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
    #[serde(default)]
    pub basis: String,
}

#[derive(Deserialize)]
pub struct AsOfQuery {
    pub as_of: String,
}

#[derive(Deserialize)]
pub struct PriorAsOfQuery {
    pub as_of: String,
    pub prior_as_of: String,
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
    let report = if q.basis == "cash" {
        ReportRepo::profit_loss_cash(&state.db, &claims.org, from, to).await?
    } else {
        ReportRepo::profit_loss(&state.db, &claims.org, from, to).await?
    };
    Ok(Json(
        serde_json::json!({ "data": report, "basis": q.basis }),
    ))
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

#[derive(Deserialize)]
pub struct Year1099Query {
    pub year: Option<i32>,
}

/// GET /api/v1/reports/1099-summary?year=YYYY
pub async fn summary_1099(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<Year1099Query>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let year = q
        .year
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().year());
    let report = ReportRepo::summary_1099(&state.db, &claims.org, year).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct ForecastQuery {
    pub from: Option<String>,
    pub days: Option<i64>,
}

/// GET /api/v1/reports/cash-flow-forecast?from=YYYY-MM-DD&days=90
pub async fn cash_flow_forecast(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ForecastQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = match q.from.as_deref() {
        Some(s) => parse_date(s)?,
        None => OffsetDateTime::now_utc().date(),
    };
    let days = q.days.unwrap_or(90).clamp(1, 365);
    let report = ReportRepo::cash_flow_forecast(&state.db, &claims.org, from, days).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub types: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/v1/search?q=&types=contacts,invoices,products,accounts&limit=10
pub async fn global_search(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    if q.q.trim().is_empty() {
        return Err(ApiError::BadRequest("q must not be empty".into()));
    }
    let limit = q.limit.unwrap_or(10).clamp(1, 20);
    let all_types = ["contacts", "invoices", "products", "accounts"];
    let type_strs: Vec<String> = match &q.types {
        None => all_types.iter().map(|s| s.to_string()).collect(),
        Some(s) => s
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| all_types.contains(&t.as_str()))
            .collect(),
    };
    let type_filter: Vec<&str> = type_strs.iter().map(String::as_str).collect();
    let hits = ReportRepo::search(&state.db, &claims.org, &q.q, &type_filter, limit).await?;
    Ok(Json(serde_json::json!({ "data": hits })))
}

#[derive(Deserialize)]
pub struct LedgerQuery {
    pub account_id: String,
    pub from: String,
    pub to: String,
}

/// GET /api/v1/reports/account-ledger?account_id=&from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn account_ledger(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<LedgerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if to < from {
        return Err(ApiError::BadRequest("to must be >= from".into()));
    }
    let ledger =
        ReportRepo::account_ledger(&state.db, &claims.org, &q.account_id, from, to).await?;
    Ok(Json(serde_json::json!({ "data": ledger })))
}

/// GET /api/v1/reports/project-profitability
pub async fn project_profitability(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let report = ReportRepo::project_profitability(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/sales-by-product?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn sales_by_product(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if to < from {
        return Err(ApiError::BadRequest("to must be >= from".into()));
    }
    let report = ReportRepo::sales_by_product(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct JobCostingQuery {
    pub from: String,
    pub to: String,
    pub project_id: Option<String>,
}

/// GET /api/v1/reports/job-costing?from=YYYY-MM-DD&to=YYYY-MM-DD[&project_id=UUID]
pub async fn job_costing(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<JobCostingQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if to < from {
        return Err(ApiError::BadRequest("to must be >= from".into()));
    }
    let report =
        ReportRepo::job_costing(&state.db, &claims.org, from, to, q.project_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/vendor-spend?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn vendor_spend(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if to < from {
        return Err(ApiError::BadRequest("to must be >= from".into()));
    }
    let report = ReportRepo::vendor_spend(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/payroll-summary?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn payroll_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if to < from {
        return Err(ApiError::BadRequest("to must be >= from".into()));
    }
    let report = ReportRepo::payroll_summary(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct PLComparisonQuery {
    pub current_from: String,
    pub current_to: String,
    pub prior_from: String,
    pub prior_to: String,
}

/// GET /api/v1/reports/pl-comparison
pub async fn pl_comparison(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PLComparisonQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let current_from = parse_date(&q.current_from)?;
    let current_to = parse_date(&q.current_to)?;
    let prior_from = parse_date(&q.prior_from)?;
    let prior_to = parse_date(&q.prior_to)?;
    let report = ReportRepo::pl_comparison(
        &state.db,
        &claims.org,
        current_from,
        current_to,
        prior_from,
        prior_to,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct GrniQuery {
    pub as_of: Option<String>,
}

/// GET /api/v1/reports/grni-accrual
pub async fn grni_accrual(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<GrniQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = match q.as_of.as_deref() {
        Some(s) => parse_date(s)?,
        None => OffsetDateTime::now_utc().date(),
    };
    let report = ReportRepo::grni_accrual(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/ar-aging-detail?as_of=YYYY-MM-DD
pub async fn ar_aging_detail(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::ar_aging_detail(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/ap-aging-detail?as_of=YYYY-MM-DD
pub async fn ap_aging_detail(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::ap_aging_detail(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/sales-by-customer?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn sales_by_customer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::sales_by_customer(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct OptAsOfQuery {
    pub as_of: Option<String>,
}

/// GET /api/v1/reports/balance-sheet-comparison?as_of=YYYY-MM-DD&prior_as_of=YYYY-MM-DD
pub async fn balance_sheet_comparison(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PriorAsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let prior_as_of = parse_date(&q.prior_as_of)?;
    if prior_as_of >= as_of {
        return Err(ApiError::BadRequest(
            "'prior_as_of' must be before 'as_of'".into(),
        ));
    }
    let report =
        ReportRepo::balance_sheet_comparison(&state.db, &claims.org, as_of, prior_as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/cash-flow-indirect?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn cash_flow_indirect(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::cash_flow_indirect(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/vat-return?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn vat_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::vat_return(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/outstanding-quotes?as_of=YYYY-MM-DD
pub async fn outstanding_quotes(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<OptAsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = match q.as_of.as_deref() {
        Some(s) => parse_date(s)?,
        None => OffsetDateTime::now_utc().date(),
    };
    let report = ReportRepo::outstanding_quotes(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/currency-exposure?as_of=YYYY-MM-DD
pub async fn currency_exposure(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<OptAsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = match q.as_of.as_deref() {
        Some(s) => parse_date(s)?,
        None => OffsetDateTime::now_utc().date(),
    };
    let report = ReportRepo::currency_exposure(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/po-spending?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn po_spending(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::po_spending(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/sales-tax-by-nexus?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn sales_tax_by_nexus(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::sales_tax_by_nexus(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/cash-receipts-journal?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn cash_receipts_journal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::cash_receipts_journal(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/cash-disbursements-journal?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn cash_disbursements_journal(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::cash_disbursements_journal(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct TrackingPLQuery {
    pub category_id: String,
    pub from: String,
    pub to: String,
}

/// GET /api/v1/reports/pl-by-tracking-category?category_id=&from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn pl_by_tracking_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<TrackingPLQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report =
        ReportRepo::pl_by_tracking_category(&state.db, &claims.org, &q.category_id, from, to)
            .await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/equity-statement?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn equity_statement(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::equity_statement(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/inventory-aging?as_of=YYYY-MM-DD
pub async fn inventory_aging(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::inventory_aging(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/customer-balances?as_of=YYYY-MM-DD
pub async fn customer_balances(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::customer_balances(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/vendor-balances?as_of=YYYY-MM-DD
pub async fn vendor_balances(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::vendor_balances(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/sales-by-rep?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn sales_by_rep(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report = ReportRepo::sales_by_rep(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// GET /api/v1/reports/project-burn?as_of=YYYY-MM-DD
pub async fn project_burn(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::project_burn(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
