use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateConsolidationElimination;
use oxidebooks_db::repos::ConsolidationEliminationRepo;
use serde::Deserialize;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct PeriodQuery {
    /// ISO date — filter eliminations with period_start >= this date
    pub period_start: Option<Date>,
    /// ISO date — filter eliminations with period_end <= this date
    pub period_end: Option<Date>,
}

/// GET /api/v1/consolidation-eliminations
pub async fn list_eliminations(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PeriodQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let eliminations = match (params.period_start, params.period_end) {
        (Some(start), Some(end)) => {
            ConsolidationEliminationRepo::list_for_period(&state.db, &claims.org, start, end)
                .await?
        }
        _ => ConsolidationEliminationRepo::list(&state.db, &claims.org).await?,
    };
    Ok(Json(serde_json::json!({ "data": eliminations })))
}

/// GET /api/v1/consolidation-eliminations/:id
pub async fn get_elimination(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let e = ConsolidationEliminationRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": e })))
}

/// POST /api/v1/consolidation-eliminations
pub async fn create_elimination(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateConsolidationElimination>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let e = ConsolidationEliminationRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": e }))))
}

/// POST /api/v1/consolidation-eliminations/:id/void
pub async fn void_elimination(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let e = ConsolidationEliminationRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": e })))
}
