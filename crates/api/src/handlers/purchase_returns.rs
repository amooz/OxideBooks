use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApprovePurchaseReturn, CreatePurchaseReturn, ShipPurchaseReturn};
use oxidebooks_db::repos::PurchaseReturnRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ReturnQuery {
    pub status: Option<String>,
}

/// GET /api/v1/purchase-returns
pub async fn list_purchase_returns(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ReturnQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let returns = PurchaseReturnRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": returns })))
}

/// GET /api/v1/purchase-returns/:id
pub async fn get_purchase_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = PurchaseReturnRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}

/// POST /api/v1/purchase-returns
pub async fn create_purchase_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePurchaseReturn>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = PurchaseReturnRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": r }))))
}

/// POST /api/v1/purchase-returns/:id/approve
pub async fn approve_purchase_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApprovePurchaseReturn>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = PurchaseReturnRepo::approve(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}

/// POST /api/v1/purchase-returns/:id/ship
pub async fn ship_purchase_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ShipPurchaseReturn>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = PurchaseReturnRepo::ship(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}
