use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateAccount, UpdateAccount};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::AccountRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/accounts
pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("accounts:read") {
        return Err(ApiError::Forbidden);
    }
    let (accounts, next_cursor) = AccountRepo::list(&state.db, &claims.org, &page).await?;
    Ok(Json(serde_json::json!({
        "data": accounts,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

/// GET /api/v1/accounts/:id
pub async fn get_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("accounts:read") {
        return Err(ApiError::Forbidden);
    }
    let account = AccountRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": account })))
}

/// POST /api/v1/accounts
pub async fn create_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateAccount>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("accounts:write") {
        return Err(ApiError::Forbidden);
    }
    let account = AccountRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": account })),
    ))
}

/// PATCH /api/v1/accounts/:id
pub async fn update_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAccount>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("accounts:write") {
        return Err(ApiError::Forbidden);
    }
    let account = AccountRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": account })))
}

/// DELETE /api/v1/accounts/:id
pub async fn delete_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("accounts:delete") {
        return Err(ApiError::Forbidden);
    }
    AccountRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
