use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateTag, UpdateTag};
use oxidebooks_db::repos::TagRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct EntityTagPath {
    pub entity_type: String,
    pub entity_id: String,
    pub tag_id: String,
}

pub async fn list_tags(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let tags = TagRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": tags })))
}

pub async fn get_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let tag = TagRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(tag)))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTag>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let tag = TagRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(tag))))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTag>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let tag = TagRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(tag)))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TagRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_entity_tags(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path((_, entity_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let tags = TagRepo::list_for_entity(&state.db, &entity_id).await?;
    Ok(Json(serde_json::json!({ "data": tags })))
}

pub async fn add_entity_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((entity_type, entity_id, tag_id)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    TagRepo::add_tag_to_entity(&state.db, &claims.org, &tag_id, &entity_id, &entity_type).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_entity_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((_, entity_id, tag_id)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    TagRepo::remove_tag_from_entity(&state.db, &claims.org, &tag_id, &entity_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
