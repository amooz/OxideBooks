use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_db::repos::ReportRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/reports/trial-balance
///
/// Returns the trial balance for the authenticated organization. Only `posted`
/// journal entries are included.
pub async fn trial_balance(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let tb = ReportRepo::trial_balance(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({
        "data": {
            "accounts": tb.accounts,
            "total_debits": tb.total_debits,
            "total_credits": tb.total_credits,
            "is_balanced": tb.is_balanced(),
        }
    })))
}
