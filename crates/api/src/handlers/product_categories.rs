use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProductCategory, UpdateProductCategory};
use oxidebooks_db::repos::ProductCategoryRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/product-categories
pub async fn list_product_categories(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("products:read") {
        return Err(ApiError::Forbidden);
    }
    let cats = ProductCategoryRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": cats })))
}

/// GET /api/v1/product-categories/:id
pub async fn get_product_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("products:read") {
        return Err(ApiError::Forbidden);
    }
    let cat = ProductCategoryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": cat })))
}

/// POST /api/v1/product-categories
pub async fn create_product_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateProductCategory>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let cat = ProductCategoryRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": cat })),
    ))
}

/// PATCH /api/v1/product-categories/:id
pub async fn update_product_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProductCategory>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let cat = ProductCategoryRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": cat })))
}

/// DELETE /api/v1/product-categories/:id
pub async fn delete_product_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    ProductCategoryRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
