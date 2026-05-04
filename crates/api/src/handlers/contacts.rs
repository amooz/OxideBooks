use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateContact, UpdateContact};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AuditRepo, ContactRepo, ReportRepo};
use serde::Deserialize;
use time::{macros::format_description, Date};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn parse_date(s: &str) -> Result<Date, ApiError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt)
        .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

#[derive(Deserialize)]
pub struct StatementQuery {
    pub from: String,
    pub to: String,
}

pub async fn contact_statement(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(q): Query<StatementQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    let statement = ReportRepo::contact_statement(&state.db, &claims.org, &id, from, to).await?;
    Ok(Json(serde_json::json!({ "data": statement })))
}

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

#[derive(Deserialize)]
pub struct MergeContactBody {
    pub discard_id: String,
}

/// POST /api/v1/contacts/:id/merge
/// Merge the `discard_id` contact into the contact at `:id`, re-pointing all references.
pub async fn merge_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(keep_id): Path<String>,
    Json(body): Json<MergeContactBody>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let contact = ContactRepo::merge(&state.db, &claims.org, &keep_id, &body.discard_id).await?;
    Ok(Json(serde_json::json!({ "data": contact })))
}
