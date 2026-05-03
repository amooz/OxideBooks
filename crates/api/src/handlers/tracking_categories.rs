use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateTrackingCategory, CreateTrackingOption, UpdateTrackingCategory, UpdateTrackingOption,
};
use oxidebooks_db::repos::TrackingCategoryRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_tracking_categories(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let cats = TrackingCategoryRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": cats })))
}

pub async fn get_tracking_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let cat = TrackingCategoryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(cat)))
}

pub async fn create_tracking_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTrackingCategory>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let cat = TrackingCategoryRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(cat))))
}

pub async fn update_tracking_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTrackingCategory>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let cat = TrackingCategoryRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(cat)))
}

pub async fn delete_tracking_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TrackingCategoryRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_tracking_option(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(category_id): Path<String>,
    Json(body): Json<CreateTrackingOption>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let opt = TrackingCategoryRepo::add_option(&state.db, &claims.org, &category_id, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(opt))))
}

pub async fn update_tracking_option(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((category_id, option_id)): Path<(String, String)>,
    Json(body): Json<UpdateTrackingOption>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let opt =
        TrackingCategoryRepo::update_option(&state.db, &claims.org, &category_id, &option_id, body)
            .await?;
    Ok(Json(serde_json::json!(opt)))
}

pub async fn delete_tracking_option(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((category_id, option_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TrackingCategoryRepo::delete_option(&state.db, &claims.org, &category_id, &option_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
