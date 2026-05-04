use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateWorkOrder, UpdateWorkOrder};
use oxidebooks_db::repos::WorkOrderRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

/// GET /api/v1/work-orders
pub async fn list_work_orders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let orders = WorkOrderRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": orders })))
}

/// GET /api/v1/work-orders/:id
pub async fn get_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/work-orders
pub async fn create_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateWorkOrder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": order })),
    ))
}

/// PATCH /api/v1/work-orders/:id
pub async fn update_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateWorkOrder>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/work-orders/:id/start
pub async fn start_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order =
        WorkOrderRepo::set_status(&state.db, &claims.org, &id, "in_progress", &["open"]).await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/work-orders/:id/hold
pub async fn hold_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::set_status(
        &state.db,
        &claims.org,
        &id,
        "on_hold",
        &["open", "in_progress"],
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/work-orders/:id/complete
pub async fn complete_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::set_status(
        &state.db,
        &claims.org,
        &id,
        "completed",
        &["open", "in_progress", "on_hold"],
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// POST /api/v1/work-orders/:id/cancel
pub async fn cancel_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let order = WorkOrderRepo::set_status(
        &state.db,
        &claims.org,
        &id,
        "cancelled",
        &["open", "in_progress", "on_hold"],
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": order })))
}

/// DELETE /api/v1/work-orders/:id
pub async fn delete_work_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    WorkOrderRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
