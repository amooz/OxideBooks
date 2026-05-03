use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateInventoryItem, InventoryAdjustment, UpdateInventoryItem};
use oxidebooks_db::repos::InventoryRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_inventory(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let items = InventoryRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

pub async fn get_inventory_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryRepo::get_by_product(&state.db, &claims.org, &product_id).await?;
    Ok(Json(serde_json::json!(item)))
}

pub async fn create_inventory_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateInventoryItem>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(item))))
}

pub async fn update_inventory_item(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
    Json(body): Json<UpdateInventoryItem>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryRepo::update(&state.db, &claims.org, &product_id, body).await?;
    Ok(Json(serde_json::json!(item)))
}

pub async fn adjust_inventory(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
    Json(body): Json<InventoryAdjustment>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = InventoryRepo::adjust(&state.db, &claims.org, &product_id, body).await?;
    Ok(Json(serde_json::json!(item)))
}

pub async fn inventory_movements(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let movements = InventoryRepo::movements(&state.db, &claims.org, &product_id).await?;
    Ok(Json(serde_json::json!({ "data": movements })))
}

pub async fn low_stock(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let items = InventoryRepo::low_stock(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/reports/inventory-valuation
pub async fn inventory_valuation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let report = InventoryRepo::valuation_report(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
