use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateJournalEntry;
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AuditRepo, TransactionRepo};
use serde::Deserialize;
use time::Date;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/transactions
pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("transactions:read") {
        return Err(ApiError::Forbidden);
    }
    let (entries, next_cursor) = TransactionRepo::list(&state.db, &claims.org, &page).await?;
    Ok(Json(serde_json::json!({
        "data": entries,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

/// GET /api/v1/transactions/:id
pub async fn get_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("transactions:read") {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// POST /api/v1/transactions
pub async fn create_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateJournalEntry>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("transactions:write") {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "create",
        "journal_entry",
        &entry.id,
        None,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": entry })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct VoidRequest {
    pub status: String,
}

/// PATCH /api/v1/transactions/:id
pub async fn void_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<VoidRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("transactions:write") {
        return Err(ApiError::Forbidden);
    }
    if body.status != "voided" {
        return Err(ApiError::BadRequest(
            "only status 'voided' is accepted".into(),
        ));
    }
    let entry = TransactionRepo::void(&state.db, &claims.org, &id).await?;
    info!(entry_id = %id, org_id = %claims.org, "journal entry voided");
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "void",
        "journal_entry",
        &id,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// POST /api/v1/transactions/:id/submit
pub async fn submit_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("transactions:write") {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::submit(&state.db, &claims.org, &claims.sub, &id).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "submit",
        "journal_entry",
        &id,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "data": entry })))
}

/// POST /api/v1/transactions/:id/approve
pub async fn approve_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let entry = TransactionRepo::approve(&state.db, &claims.org, &claims.sub, &id).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "approve",
        "journal_entry",
        &id,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "data": entry })))
}

#[derive(Debug, Deserialize)]
pub struct ReverseRequest {
    pub date: Option<String>,
}

/// POST /api/v1/transactions/:id/reverse
pub async fn reverse_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ReverseRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("transactions:write") {
        return Err(ApiError::Forbidden);
    }
    let reversal_date = if let Some(ref ds) = body.date {
        let fmt = time::format_description::parse("[year]-[month]-[day]")
            .map_err(|_| ApiError::BadRequest("invalid date format".into()))?;
        Some(
            Date::parse(ds, &fmt)
                .map_err(|_| ApiError::BadRequest("date must be YYYY-MM-DD".into()))?,
        )
    } else {
        None
    };
    let entry =
        TransactionRepo::reverse(&state.db, &claims.org, &claims.sub, &id, reversal_date).await?;
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "reverse",
        "journal_entry",
        &id,
        None,
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": entry })),
    ))
}
