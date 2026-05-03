use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateDeferredCharge, InvoiceDeferredCharges, UpdateDeferredCharge};
use oxidebooks_db::repos::DeferredChargeRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct DeferredChargeQuery {
    pub contact_id: Option<String>,
    pub status: Option<String>,
}

/// GET /api/v1/deferred-charges
pub async fn list_deferred_charges(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DeferredChargeQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let charges = DeferredChargeRepo::list(
        &state.db,
        &claims.org,
        q.contact_id.as_deref(),
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": charges })))
}

/// GET /api/v1/deferred-charges/:id
pub async fn get_deferred_charge(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let charge = DeferredChargeRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": charge })))
}

/// POST /api/v1/deferred-charges
pub async fn create_deferred_charge(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDeferredCharge>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let charge = DeferredChargeRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": charge })),
    ))
}

/// PATCH /api/v1/deferred-charges/:id
pub async fn update_deferred_charge(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateDeferredCharge>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let charge = DeferredChargeRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": charge })))
}

/// POST /api/v1/deferred-charges/:id/void
pub async fn void_deferred_charge(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let charge = DeferredChargeRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": charge })))
}

/// POST /api/v1/deferred-charges/:id/invoice
pub async fn invoice_deferred_charges(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<InvoiceDeferredCharges>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = DeferredChargeRepo::invoice_charges(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}
