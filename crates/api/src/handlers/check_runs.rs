use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateCheckRun;
use oxidebooks_db::repos::CheckRunRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/check-runs
pub async fn list_check_runs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let runs = CheckRunRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": runs })))
}

/// GET /api/v1/check-runs/:id
pub async fn get_check_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let run = CheckRunRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": run })))
}

/// GET /api/v1/check-runs/:id/items
pub async fn list_check_run_items(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let items = CheckRunRepo::list_items(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// POST /api/v1/check-runs
pub async fn create_check_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateCheckRun>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let run = CheckRunRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": run })),
    ))
}

/// POST /api/v1/check-runs/:id/print
pub async fn print_check_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let run = CheckRunRepo::print_run(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": run })))
}

/// POST /api/v1/check-runs/:id/void
pub async fn void_check_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let run = CheckRunRepo::void_run(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": run })))
}

/// POST /api/v1/check-runs/:id/items/:item_id/void
pub async fn void_check_run_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, item_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = CheckRunRepo::void_item(&state.db, &claims.org, &id, &item_id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}
