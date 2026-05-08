use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use oxidebooks_core::models::{CreateReportSchedule, UpdateReportSchedule};
use oxidebooks_db::repos::ReportScheduleRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ActiveQuery {
    #[serde(default)]
    pub active_only: bool,
}

pub async fn create_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateReportSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = ReportScheduleRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

pub async fn list_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ActiveQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedules = ReportScheduleRepo::list(&state.db, &claims.org, q.active_only).await?;
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
    let schedule = ReportScheduleRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

pub async fn update_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateReportSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = ReportScheduleRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

pub async fn delete_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ReportScheduleRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": { "deleted": true } })))
}

/// Manual trigger for a single schedule — advances last_run_at and next_run_at.
pub async fn run_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    // Verify ownership before running.
    ReportScheduleRepo::get(&state.db, &claims.org, &id).await?;
    ReportScheduleRepo::mark_run(&state.db, &id).await?;
    let schedule = ReportScheduleRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

/// Run all due schedules (admin/system endpoint).
pub async fn run_due_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let due = ReportScheduleRepo::list_due(&state.db).await?;
    let count = due.len();
    for id in &due {
        let _ = ReportScheduleRepo::mark_run(&state.db, id).await;
    }
    Ok(Json(
        serde_json::json!({ "ran": count, "schedule_ids": due }),
    ))
}
