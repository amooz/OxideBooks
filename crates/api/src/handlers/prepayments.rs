use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApplyPrepayment, CreatePrepayment};
use oxidebooks_db::repos::PrepaymentRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PrepaymentQuery {
    pub contact_id: Option<String>,
}

/// GET /api/v1/prepayments
pub async fn list_prepayments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PrepaymentQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let items = PrepaymentRepo::list(&state.db, &claims.org, q.contact_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/prepayments/:id
pub async fn get_prepayment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let item = PrepaymentRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/prepayments
pub async fn create_prepayment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePrepayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let item = PrepaymentRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// POST /api/v1/prepayments/:id/apply
pub async fn apply_prepayment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApplyPrepayment>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let item = PrepaymentRepo::apply(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/prepayments/:id/void
pub async fn void_prepayment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = PrepaymentRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}
