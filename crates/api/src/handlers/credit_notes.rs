use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApplyCreditNote, CreateCreditNote};
use oxidebooks_db::repos::CreditNoteRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/credit-notes
pub async fn list_credit_notes(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let notes = CreditNoteRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": notes })))
}

/// GET /api/v1/credit-notes/:id
pub async fn get_credit_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let note = CreditNoteRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": note })))
}

/// POST /api/v1/credit-notes
pub async fn create_credit_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateCreditNote>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let note = CreditNoteRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": note })),
    ))
}

/// POST /api/v1/credit-notes/:id/apply
pub async fn apply_credit_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApplyCreditNote>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let application = CreditNoteRepo::apply(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": application })),
    ))
}

/// POST /api/v1/credit-notes/:id/void
pub async fn void_credit_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let note = CreditNoteRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": note })))
}

/// GET /api/v1/credit-notes/:id/applications
pub async fn list_credit_note_applications(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let apps = CreditNoteRepo::list_applications(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": apps })))
}
