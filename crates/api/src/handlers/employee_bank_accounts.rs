use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateEmployeeBankAccount, UpdateEmployeeBankAccount};
use oxidebooks_db::repos::EmployeeBankAccountRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/employees/:id/bank-accounts
pub async fn list_employee_bank_accounts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(employee_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let accounts =
        EmployeeBankAccountRepo::list_for_employee(&state.db, &claims.org, &employee_id).await?;
    Ok(Json(serde_json::json!({ "data": accounts })))
}

/// POST /api/v1/employees/:id/bank-accounts
pub async fn create_employee_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(employee_id): Path<String>,
    Json(body): Json<CreateEmployeeBankAccount>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let account =
        EmployeeBankAccountRepo::create(&state.db, &claims.org, &employee_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": account })),
    ))
}

/// PATCH /api/v1/employee-bank-accounts/:id
pub async fn update_employee_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEmployeeBankAccount>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let account = EmployeeBankAccountRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": account })))
}

/// DELETE /api/v1/employee-bank-accounts/:id
pub async fn delete_employee_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    EmployeeBankAccountRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
