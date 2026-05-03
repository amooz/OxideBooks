use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateTaxGroup, UpdateTaxGroup};
use oxidebooks_db::repos::TaxGroupRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_tax_groups(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("tax_rates:read") {
        return Err(ApiError::Forbidden);
    }
    let groups = TaxGroupRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": groups })))
}

pub async fn get_tax_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("tax_rates:read") {
        return Err(ApiError::Forbidden);
    }
    let group = TaxGroupRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(group)))
}

pub async fn create_tax_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTaxGroup>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let group = TaxGroupRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(group))))
}

pub async fn update_tax_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxGroup>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let group = TaxGroupRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(group)))
}

pub async fn delete_tax_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TaxGroupRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
