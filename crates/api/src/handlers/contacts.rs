use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateContact, UpdateContact};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AuditRepo, ContactRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/contacts
pub async fn list_contacts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("contacts:read") {
        return Err(ApiError::Forbidden);
    }
    let (contacts, next_cursor) = ContactRepo::list(&state.db, &claims.org, &page).await?;
    Ok(Json(serde_json::json!({
        "data": contacts,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
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
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "create",
        "contact",
        &contact.id,
        None,
    )
    .await;
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

/// DELETE /api/v1/contacts/:id
pub async fn delete_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("contacts:delete") {
        return Err(ApiError::Forbidden);
    }
    ContactRepo::delete(&state.db, &claims.org, &id).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "delete",
        "contact",
        &id,
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
