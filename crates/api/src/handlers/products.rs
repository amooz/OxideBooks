use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProduct, SetBundleComponents, UpdateProduct};
use oxidebooks_db::repos::ProductRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ProductQuery {
    pub category_id: Option<String>,
}

/// GET /api/v1/products
pub async fn list_products(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ProductQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("settings:read") {
        return Err(ApiError::Forbidden);
    }
    let products = ProductRepo::list(&state.db, &claims.org, q.category_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": products })))
}

/// GET /api/v1/products/:id
pub async fn get_product(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("settings:read") {
        return Err(ApiError::Forbidden);
    }
    let product = ProductRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": product })))
}

/// POST /api/v1/products
pub async fn create_product(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateProduct>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let product = ProductRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": product })),
    ))
}

/// PATCH /api/v1/products/:id
pub async fn update_product(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProduct>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let product = ProductRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": product })))
}

/// DELETE /api/v1/products/:id
pub async fn delete_product(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ProductRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/v1/products/:id/bundle-components
pub async fn set_bundle_components(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<SetBundleComponents>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let product = ProductRepo::set_bundle_components(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": product })))
}
