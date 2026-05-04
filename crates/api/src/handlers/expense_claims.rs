use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateExpenseClaim, ReviewExpenseClaim, UpdateExpenseClaim};
use oxidebooks_db::repos::ExpenseClaimRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ClaimQuery {
    pub status: Option<String>,
}

/// GET /api/v1/expense-claims
pub async fn list_expense_claims(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ClaimQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let items = ExpenseClaimRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/expense-claims/:id
pub async fn get_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let item = ExpenseClaimRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/expense-claims
pub async fn create_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateExpenseClaim>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let item = ExpenseClaimRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// PATCH /api/v1/expense-claims/:id
pub async fn update_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateExpenseClaim>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let item = ExpenseClaimRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/expense-claims/:id/submit
pub async fn submit_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let item = ExpenseClaimRepo::submit(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/expense-claims/:id/approve
pub async fn approve_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ReviewExpenseClaim>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item =
        ExpenseClaimRepo::approve(&state.db, &claims.org, &id, &claims.sub, body.notes).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/expense-claims/:id/reject
pub async fn reject_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ReviewExpenseClaim>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item =
        ExpenseClaimRepo::reject(&state.db, &claims.org, &id, &claims.sub, body.notes).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/expense-claims/:id/reimburse
pub async fn reimburse_expense_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = ExpenseClaimRepo::mark_reimbursed(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}
