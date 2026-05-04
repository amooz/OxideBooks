use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePayrollTaxLiability, PayPayrollTax};
use oxidebooks_db::repos::PayrollTaxRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/payroll-tax-liabilities
pub async fn list_liabilities(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let items = PayrollTaxRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/payroll-runs/:id/tax-liabilities
pub async fn list_run_liabilities(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let items = PayrollTaxRepo::list_for_run(&state.db, &claims.org, &run_id).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/payroll-tax-liabilities/:id
pub async fn get_liability(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = PayrollTaxRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/payroll-tax-liabilities
pub async fn create_liability(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePayrollTaxLiability>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = PayrollTaxRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// POST /api/v1/payroll-tax-liabilities/:id/pay
pub async fn pay_liability(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<PayPayrollTax>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let item = PayrollTaxRepo::mark_paid(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/payroll-tax-liabilities/:id/void
pub async fn void_liability(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let item = PayrollTaxRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}
