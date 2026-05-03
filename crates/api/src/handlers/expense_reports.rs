use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{AddExpenseToReport, CreateExpenseReport, UpdateExpenseReport};
use oxidebooks_db::repos::ExpenseReportRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExpenseReportQuery {
    pub employee_id: Option<String>,
}

/// GET /api/v1/expense-reports
pub async fn list_expense_reports(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ExpenseReportQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let reports = ExpenseReportRepo::list(&state.db, &claims.org, q.employee_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": reports })))
}

/// GET /api/v1/expense-reports/:id
pub async fn get_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports
pub async fn create_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateExpenseReport>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": report })),
    ))
}

/// PATCH /api/v1/expense-reports/:id
pub async fn update_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateExpenseReport>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports/:id/expenses
pub async fn add_expense_to_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<AddExpenseToReport>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let report =
        ExpenseReportRepo::add_expense(&state.db, &claims.org, &id, &body.expense_id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports/:id/submit
pub async fn submit_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::submit(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports/:id/approve
pub async fn approve_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::approve(&state.db, &claims.org, &claims.sub, &id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports/:id/reject
pub async fn reject_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::reject(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

/// POST /api/v1/expense-reports/:id/reimburse
pub async fn reimburse_expense_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let report = ExpenseReportRepo::reimburse(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
