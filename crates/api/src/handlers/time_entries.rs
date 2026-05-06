use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    BillTimeEntries, BulkApproveTimeEntries, BulkRejectTimeEntries, CreateTimeEntry,
    RejectTimeEntry, UpdateTimeEntry,
};
use oxidebooks_db::repos::TimeEntryRepo;
use serde::Deserialize;
use time::format_description::well_known::Iso8601;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TimeEntryQuery {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

fn parse_date_opt(s: &Option<String>) -> Result<Option<Date>, ApiError> {
    match s {
        None => Ok(None),
        Some(v) => Date::parse(v, &Iso8601::DEFAULT)
            .map(Some)
            .map_err(|_| ApiError::BadRequest(format!("invalid date: {v}"))),
    }
}

pub async fn list_time_entries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<TimeEntryQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("time_entries:read") {
        return Err(ApiError::Forbidden);
    }
    // Non-admins see only their own entries
    let user_filter = if claims.is_admin() {
        q.user_id.as_deref()
    } else {
        Some(claims.sub.as_str())
    };
    let from = parse_date_opt(&q.from)?;
    let to = parse_date_opt(&q.to)?;
    let proj_filter = q.project_id.as_deref();
    let entries =
        TimeEntryRepo::list(&state.db, &claims.org, user_filter, proj_filter, from, to).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

pub async fn get_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("time_entries:read") {
        return Err(ApiError::Forbidden);
    }
    let entry = TimeEntryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && entry.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(serde_json::json!(entry)))
}

pub async fn create_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTimeEntry>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("time_entries:write") {
        return Err(ApiError::Forbidden);
    }
    let entry = TimeEntryRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(entry))))
}

pub async fn update_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTimeEntry>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("time_entries:write") {
        return Err(ApiError::Forbidden);
    }
    // Non-admins can only edit their own entries
    let existing = TimeEntryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && existing.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let entry = TimeEntryRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(entry)))
}

pub async fn delete_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("time_entries:write") {
        return Err(ApiError::Forbidden);
    }
    let existing = TimeEntryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && existing.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    TimeEntryRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn approve_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = TimeEntryRepo::approve(&state.db, &claims.org, &id, &claims.sub).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

pub async fn reject_time_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<RejectTimeEntry>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = TimeEntryRepo::reject(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

pub async fn bulk_approve_time_entries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkApproveTimeEntries>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entries = TimeEntryRepo::bulk_approve(&state.db, &claims.org, body, &claims.sub).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

pub async fn bulk_reject_time_entries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BulkRejectTimeEntries>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entries = TimeEntryRepo::bulk_reject(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

pub async fn bill_time_entries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<BillTimeEntries>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    TimeEntryRepo::bill_entries(&state.db, &claims.org, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn time_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SummaryQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("time_entries:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date_opt(&q.from)?;
    let to = parse_date_opt(&q.to)?;
    let rows = TimeEntryRepo::time_summary(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}
