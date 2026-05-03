use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePayrollEntry, CreatePayrollRun};
use oxidebooks_db::repos::PayrollRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_payroll_runs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let runs = PayrollRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": runs })))
}

pub async fn get_payroll_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let summary = PayrollRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(summary)))
}

pub async fn create_payroll_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePayrollRun>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let run = PayrollRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(run))))
}

pub async fn add_payroll_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreatePayrollEntry>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let entry = PayrollRepo::add_entry(&state.db, &claims.org, &id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(entry))))
}

pub async fn approve_payroll_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let run = PayrollRepo::approve(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(run)))
}

pub async fn pay_payroll_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let run = PayrollRepo::mark_paid(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(run)))
}
