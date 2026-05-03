use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    ConvertPrToPo, CreatePurchaseRequisition, UpdatePurchaseRequisition,
};
use oxidebooks_db::repos::PurchaseRequisitionRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PrQuery {
    pub status: Option<String>,
}

/// GET /api/v1/purchase-requisitions
pub async fn list_purchase_requisitions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PrQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let prs = PurchaseRequisitionRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": prs })))
}

/// GET /api/v1/purchase-requisitions/:id
pub async fn get_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}

/// POST /api/v1/purchase-requisitions
pub async fn create_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePurchaseRequisition>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": pr }))))
}

/// PATCH /api/v1/purchase-requisitions/:id
pub async fn update_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePurchaseRequisition>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}

/// POST /api/v1/purchase-requisitions/:id/submit
pub async fn submit_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::submit(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}

/// POST /api/v1/purchase-requisitions/:id/approve
pub async fn approve_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::approve(&state.db, &claims.org, &claims.sub, &id).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}

/// POST /api/v1/purchase-requisitions/:id/reject
pub async fn reject_purchase_requisition(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::reject(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}

/// POST /api/v1/purchase-requisitions/:id/convert
pub async fn convert_requisition_to_po(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ConvertPrToPo>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let pr = PurchaseRequisitionRepo::convert_to_po(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": pr })))
}
