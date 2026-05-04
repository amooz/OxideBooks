use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateEmployeeLoan, CreateLoanRepayment, UpdateEmployeeLoan};
use oxidebooks_db::repos::EmployeeLoanRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/employee-loans
pub async fn list_loans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let loans = EmployeeLoanRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": loans })))
}

/// GET /api/v1/employees/:id/loans
pub async fn list_employee_loans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(employee_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let loans = EmployeeLoanRepo::list_for_employee(&state.db, &claims.org, &employee_id).await?;
    Ok(Json(serde_json::json!({ "data": loans })))
}

/// GET /api/v1/employee-loans/:id
pub async fn get_loan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let loan = EmployeeLoanRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": loan })))
}

/// POST /api/v1/employee-loans
pub async fn create_loan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateEmployeeLoan>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let loan = EmployeeLoanRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": loan })),
    ))
}

/// PATCH /api/v1/employee-loans/:id
pub async fn update_loan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEmployeeLoan>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let loan = EmployeeLoanRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": loan })))
}

/// POST /api/v1/employee-loans/:id/repayments
pub async fn create_repayment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreateLoanRepayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let repayment = EmployeeLoanRepo::record_repayment(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": repayment })),
    ))
}

/// GET /api/v1/employee-loans/:id/repayments
pub async fn list_repayments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let repayments = EmployeeLoanRepo::list_repayments(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": repayments })))
}

/// POST /api/v1/employee-loans/:id/write-off
pub async fn write_off_loan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let loan = EmployeeLoanRepo::write_off(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": loan })))
}
