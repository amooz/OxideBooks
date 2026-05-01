use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateJournalEntry;
use oxidebooks_db::repos::TransactionRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/transactions
pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let entries = TransactionRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

/// GET /api/v1/transactions/:id
pub async fn get_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let entry = TransactionRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// POST /api/v1/transactions
pub async fn create_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateJournalEntry>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": entry }))))
}
