use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{CreatePrepaidExpenseSchedule, UpdatePrepaidExpenseSchedule};
use oxidebooks_db::repos::PrepaidExpenseRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/prepaid-expenses
pub async fn list_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bills:read") {
        return Err(ApiError::Forbidden);
    }
    let schedules = PrepaidExpenseRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": schedules })))
}

/// GET /api/v1/prepaid-expenses/:id
pub async fn get_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bills:read") {
        return Err(ApiError::Forbidden);
    }
    let schedule = PrepaidExpenseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// POST /api/v1/prepaid-expenses
pub async fn create_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePrepaidExpenseSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = PrepaidExpenseRepo::create(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// PATCH /api/v1/prepaid-expenses/:id
pub async fn update_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePrepaidExpenseSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = PrepaidExpenseRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// POST /api/v1/prepaid-expenses/entries/:id/recognize
pub async fn recognize_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(entry_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = PrepaidExpenseRepo::recognize(&state.db, &claims.org, &entry_id).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}
