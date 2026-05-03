use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{ResetDocSequence, UpsertDocSequence};
use oxidebooks_db::repos::DocSequenceRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/doc-sequences
pub async fn list_sequences(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let seqs = DocSequenceRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": seqs })))
}

/// PUT /api/v1/doc-sequences
pub async fn upsert_sequence(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpsertDocSequence>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let seq = DocSequenceRepo::upsert(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": seq })))
}

/// POST /api/v1/doc-sequences/:doc_type/reset
pub async fn reset_sequence(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(doc_type): Path<String>,
    Json(body): Json<ResetDocSequence>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let seq = DocSequenceRepo::reset(&state.db, &claims.org, &doc_type, body).await?;
    Ok(Json(serde_json::json!({ "data": seq })))
}
