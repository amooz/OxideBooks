use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateInventorySerialNumber, UpdateInventorySerialNumber};
use oxidebooks_db::repos::InventorySerialNumberRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct SerialQuery {
    pub product_id: Option<String>,
    pub status: Option<String>,
}

/// GET /api/v1/inventory-serial-numbers
pub async fn list_serial_numbers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SerialQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let serials = InventorySerialNumberRepo::list(
        &state.db,
        &claims.org,
        q.product_id.as_deref(),
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": serials })))
}

/// GET /api/v1/inventory-serial-numbers/:id
pub async fn get_serial_number(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let serial = InventorySerialNumberRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": serial })))
}

/// POST /api/v1/inventory-serial-numbers
pub async fn create_serial_number(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateInventorySerialNumber>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let serial = InventorySerialNumberRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": serial })),
    ))
}

/// PATCH /api/v1/inventory-serial-numbers/:id
pub async fn update_serial_number(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateInventorySerialNumber>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let serial = InventorySerialNumberRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": serial })))
}

/// DELETE /api/v1/inventory-serial-numbers/:id
pub async fn delete_serial_number(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    InventorySerialNumberRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": null })))
}
