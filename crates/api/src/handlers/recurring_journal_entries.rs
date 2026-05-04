use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateRecurringJournalEntry, UpdateRecurringJournalEntry};
use oxidebooks_db::repos::RecurringJournalEntryRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/recurring-journal-entries
pub async fn list_recurring_journal_entries(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entries = RecurringJournalEntryRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": entries })))
}

/// GET /api/v1/recurring-journal-entries/:id
pub async fn get_recurring_journal_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = RecurringJournalEntryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// POST /api/v1/recurring-journal-entries
pub async fn create_recurring_journal_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRecurringJournalEntry>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = RecurringJournalEntryRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": entry })),
    ))
}

/// PATCH /api/v1/recurring-journal-entries/:id
pub async fn update_recurring_journal_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRecurringJournalEntry>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = RecurringJournalEntryRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// DELETE /api/v1/recurring-journal-entries/:id
pub async fn delete_recurring_journal_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    RecurringJournalEntryRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/recurring-journal-entries/:id/post
pub async fn post_recurring_journal_entry(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let entry = RecurringJournalEntryRepo::post_next(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}
