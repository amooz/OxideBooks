use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateContactGroup, UpdateContactGroup};
use oxidebooks_db::repos::ContactGroupRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/contact-groups
pub async fn list_contact_groups(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let groups = ContactGroupRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": groups })))
}

/// GET /api/v1/contact-groups/:id
pub async fn get_contact_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let group = ContactGroupRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": group })))
}

/// POST /api/v1/contact-groups
pub async fn create_contact_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateContactGroup>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let group = ContactGroupRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": group })),
    ))
}

/// PATCH /api/v1/contact-groups/:id
pub async fn update_contact_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateContactGroup>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let group = ContactGroupRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": group })))
}

/// DELETE /api/v1/contact-groups/:id
pub async fn delete_contact_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    ContactGroupRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/contact-groups/:id/members
pub async fn list_group_members(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let members = ContactGroupRepo::list_members(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": members })))
}

/// POST /api/v1/contact-groups/:id/members/:contact_id
pub async fn add_group_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, contact_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    ContactGroupRepo::add_member(&state.db, &claims.org, &id, &contact_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/contact-groups/:id/members/:contact_id
pub async fn remove_group_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, contact_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    ContactGroupRepo::remove_member(&state.db, &claims.org, &id, &contact_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
