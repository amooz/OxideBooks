use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateFxRevaluation;
use oxidebooks_db::repos::FxRevaluationRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/fx/revaluations
pub async fn list_revaluations(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let revs = FxRevaluationRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": revs })))
}

/// POST /api/v1/fx/revaluations
/// Compute and post unrealized FX gain/loss for open AR/AP at the given rate.
pub async fn create_revaluation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateFxRevaluation>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rev = FxRevaluationRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rev })),
    ))
}
