use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateFixedAsset, UpdateFixedAsset};
use oxidebooks_db::repos::FixedAssetRepo;
use serde::Deserialize;
use time::format_description::well_known::Iso8601;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_fixed_assets(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let assets = FixedAssetRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": assets })))
}

pub async fn get_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(asset)))
}

pub async fn create_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateFixedAsset>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(asset))))
}

pub async fn update_fixed_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateFixedAsset>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let asset = FixedAssetRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(asset)))
}

#[derive(Debug, Deserialize)]
pub struct DepreciateBody {
    pub period_date: String,
}

pub async fn depreciate_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<DepreciateBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let period = Date::parse(&body.period_date, &Iso8601::DEFAULT)
        .map_err(|_| ApiError::BadRequest("invalid period_date, expected YYYY-MM-DD".into()))?;
    let asset = FixedAssetRepo::depreciate(&state.db, &claims.org, &id, period).await?;
    Ok(Json(serde_json::json!(asset)))
}

#[derive(Debug, Deserialize)]
pub struct DisposeBody {
    pub disposal_date: String,
}

pub async fn dispose_asset(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<DisposeBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let disposal = Date::parse(&body.disposal_date, &Iso8601::DEFAULT)
        .map_err(|_| ApiError::BadRequest("invalid disposal_date, expected YYYY-MM-DD".into()))?;
    let asset = FixedAssetRepo::dispose(&state.db, &claims.org, &id, disposal).await?;
    Ok(Json(serde_json::json!(asset)))
}

pub async fn asset_register(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("fixed_assets:read") {
        return Err(ApiError::Forbidden);
    }
    let rows = FixedAssetRepo::asset_register(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}
