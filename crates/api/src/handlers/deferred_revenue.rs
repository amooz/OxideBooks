use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateDeferredRevenueSchedule;
use oxidebooks_db::repos::DeferredRevenueRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct DrQuery {
    pub status: Option<String>,
}

pub async fn list_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DrQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedules = DeferredRevenueRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": schedules })))
}

pub async fn get_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = DeferredRevenueRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(schedule)))
}

pub async fn create_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDeferredRevenueSchedule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = DeferredRevenueRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(schedule))))
}

pub async fn recognize_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, entry_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = DeferredRevenueRepo::recognize(&state.db, &claims.org, &id, &entry_id).await?;
    Ok(Json(serde_json::json!(schedule)))
}

pub async fn cancel_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = DeferredRevenueRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(schedule)))
}
