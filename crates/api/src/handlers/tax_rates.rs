use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateTaxRate, UpdateTaxRate};
use oxidebooks_db::repos::TaxRateRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/tax-rates
pub async fn list_tax_rates(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("settings:read") {
        return Err(ApiError::Forbidden);
    }
    let rates = TaxRateRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rates })))
}

/// GET /api/v1/tax-rates/:id
pub async fn get_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("settings:read") {
        return Err(ApiError::Forbidden);
    }
    let rate = TaxRateRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": rate })))
}

/// POST /api/v1/tax-rates
pub async fn create_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTaxRate>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rate = TaxRateRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rate })),
    ))
}

/// PATCH /api/v1/tax-rates/:id
pub async fn update_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxRate>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rate = TaxRateRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": rate })))
}

/// DELETE /api/v1/tax-rates/:id
pub async fn delete_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TaxRateRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
