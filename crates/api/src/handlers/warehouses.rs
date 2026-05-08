use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreatePendingTransfer, CreateStockAdjustment, CreateWarehouse, TransferStock, UpdateWarehouse,
};
use oxidebooks_db::repos::WarehouseRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_warehouses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let warehouses = WarehouseRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": warehouses })))
}

pub async fn get_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let wh = WarehouseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": wh })))
}

pub async fn create_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateWarehouse>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let wh = WarehouseRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": wh }))))
}

pub async fn update_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateWarehouse>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let wh = WarehouseRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": wh })))
}

pub async fn delete_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    WarehouseRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_warehouse_stock(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let stock = WarehouseRepo::stock(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": stock })))
}

pub async fn transfer_stock(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<TransferStock>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let transfer = WarehouseRepo::transfer(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": transfer })))
}

// ── Transfer lifecycle ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TransferListQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn list_transfers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<TransferListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let limit = q.limit.clamp(1, 200);
    let transfers =
        WarehouseRepo::list_transfers(&state.db, &claims.org, q.status.as_deref(), limit).await?;
    Ok(Json(serde_json::json!({ "data": transfers })))
}

pub async fn get_transfer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let transfer = WarehouseRepo::get_transfer(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": transfer })))
}

pub async fn create_pending_transfer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePendingTransfer>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let transfer = WarehouseRepo::create_pending_transfer(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": transfer })),
    ))
}

pub async fn receive_transfer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let transfer = WarehouseRepo::receive_transfer(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": transfer })))
}

pub async fn cancel_transfer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let transfer = WarehouseRepo::cancel_transfer(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": transfer })))
}

// ── Stock adjustments ──────────────────────────────────────────────────────────

pub async fn adjust_stock(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreateStockAdjustment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let adj = WarehouseRepo::adjust_stock(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": adj })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct AdjustmentQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

pub async fn list_adjustments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Query(q): Query<AdjustmentQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let limit = q.limit.clamp(1, 200);
    let adjs = WarehouseRepo::list_adjustments(&state.db, &claims.org, &id, limit).await?;
    Ok(Json(serde_json::json!({ "data": adjs })))
}

// ── Cross-location summary ─────────────────────────────────────────────────────

pub async fn stock_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let summary = WarehouseRepo::stock_summary(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": summary })))
}
