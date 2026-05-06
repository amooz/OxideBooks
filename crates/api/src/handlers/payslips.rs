use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreatePayslip;
use oxidebooks_db::repos::PayslipRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// POST /api/v1/payroll-runs/:id/payslips
pub async fn create_payslip(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(run_id): Path<String>,
    Json(body): Json<CreatePayslip>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let payslip = PayslipRepo::create(&state.db, &claims.org, &run_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": payslip })),
    ))
}

/// GET /api/v1/payroll-runs/:id/payslips
pub async fn list_payslips(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payslips = PayslipRepo::list_by_run(&state.db, &claims.org, &run_id).await?;
    Ok(Json(serde_json::json!({ "data": payslips })))
}

/// GET /api/v1/payslips/:id
pub async fn get_payslip(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payslip = PayslipRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": payslip })))
}

/// POST /api/v1/payslips/:id/publish
pub async fn publish_payslip(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let payslip = PayslipRepo::publish(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": payslip })))
}

/// GET /api/v1/employees/:id/payslips
pub async fn list_employee_payslips(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(employee_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payslips = PayslipRepo::list_by_employee(&state.db, &claims.org, &employee_id).await?;
    Ok(Json(serde_json::json!({ "data": payslips })))
}
