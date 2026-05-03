use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::CreateJournalEntry;
use oxidebooks_db::repos::TransactionRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// POST /api/v1/opening-balances
///
/// Creates a balanced opening-balance journal entry and immediately posts it
/// (bypasses the draft → submit → approve workflow). Admin-only.
pub async fn set_opening_balances(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateJournalEntry>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::create_posted(&state.db, &claims.org, &claims.sub, body).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// GET /api/v1/opening-balances
///
/// Returns the most recent posted journal entry tagged as an opening-balance entry
/// (reference = 'OPENING_BALANCE').
pub async fn get_opening_balances(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::get_opening_balance(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}
