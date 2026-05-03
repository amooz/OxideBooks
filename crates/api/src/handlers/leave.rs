use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateLeaveRequest, CreateLeaveType, UpdateLeaveType};
use oxidebooks_db::repos::LeaveRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

// ── Leave Types ───────────────────────────────────────────────────────────────

/// GET /api/v1/leave-types
pub async fn list_leave_types(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let types = LeaveRepo::list_types(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": types })))
}

/// POST /api/v1/leave-types
pub async fn create_leave_type(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateLeaveType>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let lt = LeaveRepo::create_type(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": lt }))))
}

/// PATCH /api/v1/leave-types/:id
pub async fn update_leave_type(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateLeaveType>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let lt = LeaveRepo::update_type(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": lt })))
}

/// DELETE /api/v1/leave-types/:id
pub async fn delete_leave_type(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    LeaveRepo::delete_type(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Leave Requests ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LeaveRequestQuery {
    pub employee_id: Option<String>,
}

/// GET /api/v1/leave-requests
pub async fn list_leave_requests(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<LeaveRequestQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let requests =
        LeaveRepo::list_requests(&state.db, &claims.org, q.employee_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": requests })))
}

/// POST /api/v1/leave-requests
pub async fn create_leave_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateLeaveRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let req = LeaveRepo::create_request(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": req })),
    ))
}

/// POST /api/v1/leave-requests/:id/approve
pub async fn approve_leave_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let req = LeaveRepo::update_request_status(
        &state.db,
        &claims.org,
        &id,
        "approved",
        Some(&claims.sub),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": req })))
}

/// POST /api/v1/leave-requests/:id/reject
pub async fn reject_leave_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let req =
        LeaveRepo::update_request_status(&state.db, &claims.org, &id, "rejected", None).await?;
    Ok(Json(serde_json::json!({ "data": req })))
}

/// POST /api/v1/leave-requests/:id/cancel
pub async fn cancel_leave_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let req =
        LeaveRepo::update_request_status(&state.db, &claims.org, &id, "cancelled", None).await?;
    Ok(Json(serde_json::json!({ "data": req })))
}
