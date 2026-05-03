use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{CreateWarehouse, TransferStock, UpdateWarehouse};
use oxidebooks_db::repos::WarehouseRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/warehouses
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

/// GET /api/v1/warehouses/:id
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

/// POST /api/v1/warehouses
pub async fn create_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateWarehouse>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let wh = WarehouseRepo::create(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": wh })))
}

/// PATCH /api/v1/warehouses/:id
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

/// DELETE /api/v1/warehouses/:id
pub async fn delete_warehouse(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    WarehouseRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /api/v1/warehouses/:id/stock
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

/// POST /api/v1/warehouses/transfer
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
