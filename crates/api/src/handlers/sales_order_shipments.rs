use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateSalesOrderShipment;
use oxidebooks_db::repos::SalesOrderShipmentRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/sales-orders/:id/shipments
pub async fn list_shipments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(so_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let shipments = SalesOrderShipmentRepo::list_for_order(&state.db, &claims.org, &so_id).await?;
    Ok(Json(serde_json::json!({ "data": shipments })))
}

/// GET /api/v1/so-shipments/:id
pub async fn get_shipment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let shipment = SalesOrderShipmentRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": shipment })))
}

/// POST /api/v1/sales-orders/:id/shipments
pub async fn create_shipment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(so_id): Path<String>,
    Json(body): Json<CreateSalesOrderShipment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let shipment = SalesOrderShipmentRepo::create(&state.db, &claims.org, &so_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": shipment })),
    ))
}
