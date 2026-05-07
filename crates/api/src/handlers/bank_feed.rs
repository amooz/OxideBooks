use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ImportBankFeed, MatchBankFeedTransaction};
use oxidebooks_db::repos::BankFeedRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    pub status: Option<String>,
}

/// POST /api/v1/bank-accounts/:id/feed/import
pub async fn import_feed(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ImportBankFeed>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rows = BankFeedRepo::import(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rows, "imported": rows.len() })),
    ))
}

/// GET /api/v1/bank-accounts/:id/feed
pub async fn list_feed(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Query(q): Query<FeedQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rows = BankFeedRepo::list(&state.db, &claims.org, &id, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

/// POST /api/v1/bank-feed/:id/match
pub async fn match_feed_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<MatchBankFeedTransaction>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let txn = BankFeedRepo::match_transaction(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": txn })))
}

/// POST /api/v1/bank-feed/:id/ignore
pub async fn ignore_feed_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let txn = BankFeedRepo::ignore(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": txn })))
}

/// POST /api/v1/bank-accounts/:id/feed/auto-match
pub async fn auto_match_feed(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let result = BankFeedRepo::auto_match(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": result })))
}
