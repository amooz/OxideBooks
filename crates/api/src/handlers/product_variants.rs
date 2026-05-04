use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProductVariant, UpdateProductVariant};
use oxidebooks_db::repos::ProductVariantRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct VariantListQuery {
    pub active_only: Option<bool>,
}

pub async fn list_variants(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
    Query(q): Query<VariantListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let active_only = q.active_only.unwrap_or(true);
    let variants =
        ProductVariantRepo::list_for_product(&state.db, &claims.org, &product_id, active_only)
            .await?;
    Ok(Json(serde_json::json!({ "data": variants })))
}

pub async fn get_variant(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let variant =
        ProductVariantRepo::get_by_id(&state.db, &claims.org, &product_id, &variant_id).await?;
    Ok(Json(serde_json::json!({ "data": variant })))
}

pub async fn create_variant(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(product_id): Path<String>,
    Json(body): Json<CreateProductVariant>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let variant = ProductVariantRepo::create(&state.db, &claims.org, &product_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": variant })),
    ))
}

pub async fn update_variant(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((product_id, variant_id)): Path<(String, String)>,
    Json(body): Json<UpdateProductVariant>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let variant =
        ProductVariantRepo::update(&state.db, &claims.org, &product_id, &variant_id, body).await?;
    Ok(Json(serde_json::json!({ "data": variant })))
}

pub async fn delete_variant(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ProductVariantRepo::delete(&state.db, &claims.org, &product_id, &variant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
