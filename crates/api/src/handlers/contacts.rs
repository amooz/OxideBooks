use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateContact, UpdateContact};
use oxidebooks_db::repos::ContactRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/contacts
pub async fn list_contacts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let contacts = ContactRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": contacts })))
}

/// GET /api/v1/contacts/:id
pub async fn get_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let contact = ContactRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": contact })))
}

/// POST /api/v1/contacts
pub async fn create_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateContact>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("contacts:write") {
        return Err(ApiError::Forbidden);
    }
    let contact = ContactRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": contact })),
    ))
}

/// PATCH /api/v1/contacts/:id
pub async fn update_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateContact>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:write") {
        return Err(ApiError::Forbidden);
    }
    let contact = ContactRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": contact })))
}
