use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use oxidebooks_core::models::{CreateInventoryLot, UpdateInventoryLot};
use oxidebooks_db::repos::InventoryLotRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/inventory/:item_id/lots
pub async fn list_lots(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(item_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let lots = InventoryLotRepo::list(&state.db, &claims.org, &item_id).await?;
    Ok(Json(serde_json::json!({ "data": lots })))
}

/// POST /api/v1/inventory/:item_id/lots
pub async fn create_lot(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(item_id): Path<String>,
    Json(mut body): Json<CreateInventoryLot>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    body.item_id = item_id;
    let lot = InventoryLotRepo::create(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": lot })))
}

/// GET /api/v1/inventory/lots/:id
pub async fn get_lot(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let lot = InventoryLotRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": lot })))
}

/// PATCH /api/v1/inventory/lots/:id
pub async fn update_lot(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateInventoryLot>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let lot = InventoryLotRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": lot })))
}

#[derive(Deserialize)]
pub struct ExpiringQuery {
    pub days: Option<i64>,
}

/// GET /api/v1/inventory/lots/expiring?days=30
pub async fn list_expiring(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ExpiringQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("inventory:read") {
        return Err(ApiError::Forbidden);
    }
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let lots = InventoryLotRepo::list_expiring(&state.db, &claims.org, days).await?;
    Ok(Json(serde_json::json!({ "data": lots })))
}
