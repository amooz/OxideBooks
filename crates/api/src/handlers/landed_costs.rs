use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::CreateLandedCost;
use oxidebooks_db::repos::LandedCostRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/goods-receipts/:id/landed-costs
pub async fn list_landed_costs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(grn_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("purchases:read") {
        return Err(ApiError::Forbidden);
    }
    let costs = LandedCostRepo::list(&state.db, &claims.org, &grn_id).await?;
    Ok(Json(serde_json::json!({ "data": costs })))
}

/// POST /api/v1/goods-receipts/:id/landed-costs
pub async fn create_landed_cost(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(grn_id): Path<String>,
    Json(body): Json<CreateLandedCost>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let cost = LandedCostRepo::create(&state.db, &claims.org, &grn_id, body).await?;
    Ok(Json(serde_json::json!({ "data": cost })))
}
