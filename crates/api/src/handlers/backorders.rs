use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateBackorder, CreateDropShipRequest, FulfillBackorder, UpdateDropShipRequest,
};
use oxidebooks_db::repos::BackorderRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

// ── Backorders ─────────────────────────────────────────────────────────────────

pub async fn list_backorders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("sales_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let limit = q.limit.clamp(1, 200);
    let backorders =
        BackorderRepo::list(&state.db, &claims.org, q.status.as_deref(), limit).await?;
    Ok(Json(serde_json::json!({ "data": backorders })))
}

pub async fn get_backorder(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("sales_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let bo = BackorderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": bo })))
}

pub async fn create_backorder(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBackorder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bo = BackorderRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": bo }))))
}

pub async fn fulfill_backorder(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<FulfillBackorder>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bo = BackorderRepo::fulfill(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": bo })))
}

pub async fn cancel_backorder(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bo = BackorderRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": bo })))
}

// ── Drop-ship requests ─────────────────────────────────────────────────────────

pub async fn list_drop_ships(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let limit = q.limit.clamp(1, 200);
    let requests =
        BackorderRepo::list_drop_ships(&state.db, &claims.org, q.status.as_deref(), limit).await?;
    Ok(Json(serde_json::json!({ "data": requests })))
}

pub async fn get_drop_ship(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let ds = BackorderRepo::get_drop_ship(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": ds })))
}

pub async fn create_drop_ship(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDropShipRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let ds = BackorderRepo::create_drop_ship(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": ds }))))
}

pub async fn update_drop_ship(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDropShipRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let ds = BackorderRepo::update_drop_ship(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": ds })))
}
