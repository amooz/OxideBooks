use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApplyVendorCredit, CreateVendorCredit};
use oxidebooks_db::repos::VendorCreditRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct VendorCreditQuery {
    pub contact_id: Option<String>,
}

/// GET /api/v1/vendor-credits
pub async fn list_vendor_credits(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<VendorCreditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bills:read") {
        return Err(ApiError::Forbidden);
    }
    let credits = VendorCreditRepo::list(&state.db, &claims.org, q.contact_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": credits })))
}

/// GET /api/v1/vendor-credits/:id
pub async fn get_vendor_credit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bills:read") {
        return Err(ApiError::Forbidden);
    }
    let credit = VendorCreditRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": credit })))
}

/// POST /api/v1/vendor-credits
pub async fn create_vendor_credit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateVendorCredit>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("bills:write") {
        return Err(ApiError::Forbidden);
    }
    let credit = VendorCreditRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": credit })),
    ))
}

/// POST /api/v1/vendor-credits/:id/void
pub async fn void_vendor_credit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let credit = VendorCreditRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": credit })))
}

/// POST /api/v1/vendor-credits/:id/apply
pub async fn apply_vendor_credit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApplyVendorCredit>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let app = VendorCreditRepo::apply(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": app })))
}

/// GET /api/v1/vendor-credits/:id/applications
pub async fn list_credit_applications(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bills:read") {
        return Err(ApiError::Forbidden);
    }
    let apps = VendorCreditRepo::list_applications(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": apps })))
}
