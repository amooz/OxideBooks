use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::CreateGrn;
use oxidebooks_db::repos::GrnRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/purchase-orders/:id/receipts
pub async fn list_receipts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(po_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchases:read") {
        return Err(ApiError::Forbidden);
    }
    let grns = GrnRepo::list(&state.db, &claims.org, &po_id).await?;
    Ok(Json(serde_json::json!({ "data": grns })))
}

/// POST /api/v1/purchase-orders/:id/receipts
pub async fn create_receipt(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(po_id): Path<String>,
    Json(mut body): Json<CreateGrn>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    body.purchase_order_id = po_id;
    let grn = GrnRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok(Json(serde_json::json!({ "data": grn })))
}

/// GET /api/v1/goods-receipts/:id
pub async fn get_receipt(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchases:read") {
        return Err(ApiError::Forbidden);
    }
    let grn = GrnRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": grn })))
}

/// POST /api/v1/goods-receipts/:id/post
pub async fn post_receipt(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let grn = GrnRepo::post(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": grn })))
}
