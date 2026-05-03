use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateClosedPeriod;
use oxidebooks_db::repos::ClosedPeriodRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_closed_periods(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let periods = ClosedPeriodRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": periods })))
}

pub async fn close_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateClosedPeriod>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let period = ClosedPeriodRepo::close(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(period))))
}

pub async fn reopen_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ClosedPeriodRepo::reopen(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
