use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreatePoLine, CreatePurchaseOrder, ReceivePoLine, UpdatePurchaseOrder,
};
use oxidebooks_db::repos::PurchaseOrderRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PoQuery {
    pub status: Option<String>,
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PoQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let pos = PurchaseOrderRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": pos })))
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchase_orders:read") {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePurchaseOrder>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(po))))
}

pub async fn update_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePurchaseOrder>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn delete_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    PurchaseOrderRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn receive_purchase_order(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<Vec<ReceivePoLine>>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::receive(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(po)))
}

pub async fn add_po_line(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreatePoLine>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let po = PurchaseOrderRepo::add_line(&state.db, &claims.org, &id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(po))))
}
