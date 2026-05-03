use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateCustomFieldDefinition, SetCustomFieldValue, UpdateCustomFieldDefinition,
};
use oxidebooks_db::repos::CustomFieldRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CfQuery {
    pub entity_type: Option<String>,
}

pub async fn list_custom_fields(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<CfQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("custom_fields:read") {
        return Err(ApiError::Forbidden);
    }
    let defs =
        CustomFieldRepo::list_definitions(&state.db, &claims.org, q.entity_type.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": defs })))
}

pub async fn get_custom_field(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("custom_fields:read") {
        return Err(ApiError::Forbidden);
    }
    let def = CustomFieldRepo::get_definition(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(def)))
}

pub async fn create_custom_field(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateCustomFieldDefinition>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let def = CustomFieldRepo::create_definition(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(def))))
}

pub async fn update_custom_field(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCustomFieldDefinition>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let def = CustomFieldRepo::update_definition(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(def)))
}

pub async fn delete_custom_field(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    CustomFieldRepo::delete_definition(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct EntityPath {
    pub entity_type: String,
    pub entity_id: String,
}

pub async fn get_entity_custom_fields(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(p): Path<EntityPath>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("custom_fields:read") {
        return Err(ApiError::Forbidden);
    }
    let values =
        CustomFieldRepo::get_values(&state.db, &claims.org, &p.entity_type, &p.entity_id).await?;
    Ok(Json(serde_json::json!({ "data": values })))
}

pub async fn set_entity_custom_fields(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(p): Path<EntityPath>,
    Json(body): Json<Vec<SetCustomFieldValue>>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    CustomFieldRepo::set_values(&state.db, &claims.org, &p.entity_id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}
