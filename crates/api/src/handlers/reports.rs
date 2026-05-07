use axum::{
    extract::{Extension, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use oxidebooks_db::repos::ReportRepo;
use serde::Deserialize;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

use oxidebooks_core::models::ReportLine;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
    xlsx::{build_xlsx, XLSX_CONTENT_TYPE},
};

fn xlsx_response(bytes: Vec<u8>, filename: &str) -> Response {
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(XLSX_CONTENT_TYPE),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
                    .unwrap_or_else(|_| {
                        axum::http::HeaderValue::from_static("attachment; filename=\"report.xlsx\"")
                    }),
            ),
        ],
        bytes,
    )
        .into_response()
}

fn csv_response(csv: String, filename: &str) -> Response {
    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
                    .unwrap_or_else(|_| {
                        axum::http::HeaderValue::from_static("attachment; filename=\"report.csv\"")
                    }),
            ),
        ],
        csv,
    )
        .into_response()
}

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
    pub format: Option<String>,
    pub compare: Option<String>,
}

#[derive(Deserialize)]
pub struct AsOfQuery {
    pub as_of: String,
    #[serde(default)]
    pub basis: String,
    pub format: Option<String>,
}

#[derive(Deserialize)]
pub struct PriorAsOfQuery {
    pub as_of: String,
    pub prior_as_of: String,
}

#[derive(Deserialize)]
pub struct FormatQuery {
    pub format: Option<String>,
}

/// GET /api/v1/reports/trial-balance[?format=xlsx|csv]
pub async fn trial_balance(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<FormatQuery>,
) -> ApiResult<Response> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let tb = ReportRepo::trial_balance(&state.db, &claims.org).await?;
    let fmt = q.format.as_deref().unwrap_or("json");

    if fmt == "xlsx" || fmt == "csv" {
        let headers = &[
            "Account Code",
            "Account Name",
            "Account Type",
            "Debit",
            "Credit",
            "Balance",
        ];
        let rows: Vec<Vec<String>> = tb
            .accounts
            .iter()
            .map(|a| {
                vec![
                    a.account_code.clone(),
                    a.account_name.clone(),
                    format!("{:?}", a.account_type),
                    a.debit_total.to_string(),
                    a.credit_total.to_string(),
                    a.balance().to_string(),
                ]
            })
            .collect();

        if fmt == "xlsx" {
            let bytes = build_xlsx("Trial Balance", headers, &rows);
            return Ok(xlsx_response(bytes, "trial-balance.xlsx"));
        } else {
            let mut wtr = csv::Writer::from_writer(vec![]);
            wtr.write_record(headers).ok();
            for row in &rows {
                wtr.write_record(row).ok();
            }
            let csv = String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();
            return Ok(csv_response(csv, "trial-balance.csv"));
        }
    }

    Ok(Json(serde_json::json!({
        "data": {
            "accounts": tb.accounts,
            "total_debits": tb.total_debits,
            "total_credits": tb.total_credits,
            "is_balanced": tb.is_balanced(),
        }
    }))
    .into_response())
}

/// GET /api/v1/reports/profit-loss?from=YYYY-MM-DD&to=YYYY-MM-DD[&compare=prior_year][&format=xlsx|csv]
pub async fn profit_loss(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DateRangeQuery>,
) -> ApiResult<Response> {
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

    let compare = q.compare.as_deref() == Some("prior_year");
    let fmt = q.format.as_deref().unwrap_or("json");

    // Fetch prior year if requested
    let prior = if compare {
        use time::Duration;
        let py_from = from - Duration::days(365);
        let py_to = to - Duration::days(365);
        let r = if q.basis == "cash" {
            ReportRepo::profit_loss_cash(&state.db, &claims.org, py_from, py_to).await?
        } else {
            ReportRepo::profit_loss(&state.db, &claims.org, py_from, py_to).await?
        };
        Some(r)
    } else {
        None
    };

    if fmt == "xlsx" || fmt == "csv" {
        let has_prior = prior.is_some();
        let mut headers: Vec<&str> = vec!["Section", "Account Code", "Account Name", "Amount"];
        if has_prior {
            headers.push("Prior Year Amount");
        }

        let mut rows: Vec<Vec<String>> = Vec::new();
        let build_section = |section_name: &str, lines: &[ReportLine]| {
            lines
                .iter()
                .map(|l| {
                    vec![
                        section_name.to_string(),
                        l.account_code.clone(),
                        l.account_name.clone(),
                        l.amount.to_string(),
                    ]
                })
                .collect::<Vec<_>>()
        };

        let mut rev_rows = build_section("Revenue", &report.revenue.accounts);
        let mut exp_rows = build_section("Expenses", &report.expenses.accounts);

        if let Some(ref p) = prior {
            // Merge prior year amounts by account_id
            use std::collections::HashMap;
            let prior_rev: HashMap<&str, i64> = p
                .revenue
                .accounts
                .iter()
                .map(|l| (l.account_id.as_str(), l.amount))
                .collect();
            let prior_exp: HashMap<&str, i64> = p
                .expenses
                .accounts
                .iter()
                .map(|l| (l.account_id.as_str(), l.amount))
                .collect();

            for (row, line) in rev_rows.iter_mut().zip(report.revenue.accounts.iter()) {
                row.push(
                    prior_rev
                        .get(line.account_id.as_str())
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                );
            }
            for (row, line) in exp_rows.iter_mut().zip(report.expenses.accounts.iter()) {
                row.push(
                    prior_exp
                        .get(line.account_id.as_str())
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                );
            }
        }

        rows.extend(rev_rows);
        rows.extend(exp_rows);

        // Net income row
        let mut net_row = vec![
            "Net Income".to_string(),
            String::new(),
            String::new(),
            report.net_income.to_string(),
        ];
        if let Some(ref p) = prior {
            net_row.push(p.net_income.to_string());
        }
        rows.push(net_row);

        let headers_ref: Vec<&str> = headers.clone();
        if fmt == "xlsx" {
            let bytes = build_xlsx("Profit & Loss", &headers_ref, &rows);
            return Ok(xlsx_response(bytes, "profit-loss.xlsx"));
        } else {
            let mut wtr = csv::Writer::from_writer(vec![]);
            wtr.write_record(&headers_ref).ok();
            for row in &rows {
                wtr.write_record(row).ok();
            }
            let csv = String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();
            return Ok(csv_response(csv, "profit-loss.csv"));
        }
    }

    if let Some(prior_report) = prior {
        Ok(Json(serde_json::json!({
            "data": report,
            "prior_year": prior_report,
            "basis": q.basis,
        }))
        .into_response())
    } else {
        Ok(Json(serde_json::json!({ "data": report, "basis": q.basis })).into_response())
    }
}

/// GET /api/v1/reports/balance-sheet?as_of=YYYY-MM-DD[&basis=cash][&format=xlsx|csv]
pub async fn balance_sheet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Response> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let fmt = q.format.as_deref().unwrap_or("json");

    if q.basis == "cash" {
        let report = ReportRepo::cash_basis_balance_sheet(&state.db, &claims.org, as_of).await?;
        if fmt == "xlsx" || fmt == "csv" {
            let headers = &["Section", "Account Code", "Account Name", "Amount"];
            let mut rows: Vec<Vec<String>> = Vec::new();
            for a in &report.cash.accounts {
                rows.push(vec![
                    "Cash".into(),
                    a.account_code.clone(),
                    a.account_name.clone(),
                    a.amount.to_string(),
                ]);
            }
            for e in &report.equity.accounts {
                rows.push(vec![
                    "Equity".into(),
                    e.account_code.clone(),
                    e.account_name.clone(),
                    e.amount.to_string(),
                ]);
            }
            if fmt == "xlsx" {
                let bytes = build_xlsx("Balance Sheet (Cash)", headers, &rows);
                return Ok(xlsx_response(bytes, "balance-sheet.xlsx"));
            } else {
                let mut wtr = csv::Writer::from_writer(vec![]);
                wtr.write_record(headers).ok();
                for row in &rows {
                    wtr.write_record(row).ok();
                }
                let csv =
                    String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();
                return Ok(csv_response(csv, "balance-sheet.csv"));
            }
        }
        return Ok(Json(serde_json::json!({ "data": report, "basis": "cash" })).into_response());
    }

    let report = ReportRepo::balance_sheet(&state.db, &claims.org, as_of).await?;
    if fmt == "xlsx" || fmt == "csv" {
        let headers = &["Section", "Account Code", "Account Name", "Amount"];
        let mut rows: Vec<Vec<String>> = Vec::new();
        for a in &report.assets.accounts {
            rows.push(vec![
                "Assets".into(),
                a.account_code.clone(),
                a.account_name.clone(),
                a.amount.to_string(),
            ]);
        }
        for l in &report.liabilities.accounts {
            rows.push(vec![
                "Liabilities".into(),
                l.account_code.clone(),
                l.account_name.clone(),
                l.amount.to_string(),
            ]);
        }
        for e in &report.equity.accounts {
            rows.push(vec![
                "Equity".into(),
                e.account_code.clone(),
                e.account_name.clone(),
                e.amount.to_string(),
            ]);
        }
        if fmt == "xlsx" {
            let bytes = build_xlsx("Balance Sheet", headers, &rows);
            return Ok(xlsx_response(bytes, "balance-sheet.xlsx"));
        } else {
            let mut wtr = csv::Writer::from_writer(vec![]);
            wtr.write_record(headers).ok();
            for row in &rows {
                wtr.write_record(row).ok();
            }
            let csv = String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default();
            return Ok(csv_response(csv, "balance-sheet.csv"));
        }
    }
    Ok(Json(serde_json::json!({ "data": report, "basis": "accrual" })).into_response())
}

/// GET /api/v1/reports/cash-basis-pl?from=YYYY-MM-DD&to=YYYY-MM-DD
pub async fn cash_basis_pl(
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
    let report = ReportRepo::profit_loss_cash(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report, "basis": "cash" })))
}

/// GET /api/v1/reports/cash-basis-balance-sheet?as_of=YYYY-MM-DD
pub async fn cash_basis_balance_sheet(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AsOfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let as_of = parse_date(&q.as_of)?;
    let report = ReportRepo::cash_basis_balance_sheet(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": report, "basis": "cash" })))
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
