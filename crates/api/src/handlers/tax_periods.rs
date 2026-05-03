use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateTaxPeriod, FileTaxPeriod};
use oxidebooks_db::repos::TaxPeriodRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/tax-periods
pub async fn list_tax_periods(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let periods = TaxPeriodRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": periods })))
}

/// GET /api/v1/tax-periods/:id
pub async fn get_tax_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let period = TaxPeriodRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": period })))
}

/// POST /api/v1/tax-periods
pub async fn create_tax_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTaxPeriod>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let period = TaxPeriodRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": period })),
    ))
}

/// POST /api/v1/tax-periods/:id/file
pub async fn file_tax_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<FileTaxPeriod>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let period = TaxPeriodRepo::file(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": period })))
}

/// POST /api/v1/tax-periods/:id/lock
pub async fn lock_tax_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let period = TaxPeriodRepo::lock(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": period })))
}

/// DELETE /api/v1/tax-periods/:id
pub async fn delete_tax_period(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TaxPeriodRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
