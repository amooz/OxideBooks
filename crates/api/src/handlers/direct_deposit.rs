use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateDirectDepositBatch, MarkBatchSent};
use oxidebooks_db::repos::DirectDepositRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/direct-deposit-batches
pub async fn list_batches(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let items = DirectDepositRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/direct-deposit-batches/:id
pub async fn get_batch(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = DirectDepositRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/direct-deposit-batches
pub async fn create_batch(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDirectDepositBatch>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.entries.is_empty() {
        return Err(ApiError::BadRequest("entries must not be empty".into()));
    }
    let item = DirectDepositRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// POST /api/v1/direct-deposit-batches/:id/send
/// Mark the batch as sent (transitions pending → sent).
pub async fn mark_sent(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<MarkBatchSent>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = DirectDepositRepo::mark_sent(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// DELETE /api/v1/direct-deposit-batches/:id
/// Only pending batches may be deleted.
pub async fn delete_batch(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    DirectDepositRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
