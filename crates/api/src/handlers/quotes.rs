use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ConvertQuoteToInvoice, CreateQuote, UpdateQuote};
use oxidebooks_db::repos::QuoteRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct QuoteQuery {
    pub status: Option<String>,
    pub contact_id: Option<String>,
}

/// GET /api/v1/quotes
pub async fn list_quotes(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<QuoteQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quotes = QuoteRepo::list(
        &state.db,
        &claims.org,
        q.status.as_deref(),
        q.contact_id.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": quotes })))
}

/// GET /api/v1/quotes/:id
pub async fn get_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}

/// POST /api/v1/quotes
pub async fn create_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateQuote>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": quote })),
    ))
}

/// PATCH /api/v1/quotes/:id
pub async fn update_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateQuote>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}

/// DELETE /api/v1/quotes/:id
pub async fn delete_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    QuoteRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/quotes/:id/send
pub async fn send_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::send(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}

/// POST /api/v1/quotes/:id/accept
pub async fn accept_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::accept(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}

/// POST /api/v1/quotes/:id/decline
pub async fn decline_quote(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::decline(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}

/// POST /api/v1/quotes/:id/convert-to-invoice
pub async fn convert_quote_to_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ConvertQuoteToInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let quote = QuoteRepo::convert_to_invoice(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": quote })))
}
