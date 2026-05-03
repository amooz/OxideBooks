use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateExpense, ExpenseStatus, UpdateExpense};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::ExpenseRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExpenseQuery {
    #[serde(flatten)]
    pub page: PageParams,
    pub user_id: Option<String>,
    pub status: Option<String>,
}

/// GET /api/v1/expenses
pub async fn list_expenses(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ExpenseQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    // Non-admins can only see their own expenses
    let user_filter = if claims.is_admin() {
        q.user_id.as_deref()
    } else {
        Some(claims.sub.as_str())
    };
    let (expenses, next_cursor) = ExpenseRepo::list(
        &state.db,
        &claims.org,
        &q.page,
        user_filter,
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({
        "data": expenses,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

/// GET /api/v1/expenses/:id
pub async fn get_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let expense = ExpenseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && expense.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(serde_json::json!({ "data": expense })))
}

/// POST /api/v1/expenses
pub async fn create_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateExpense>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let expense = ExpenseRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": expense })),
    ))
}

/// PATCH /api/v1/expenses/:id
pub async fn update_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateExpense>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let expense = ExpenseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && expense.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let updated = ExpenseRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

/// POST /api/v1/expenses/:id/submit
pub async fn submit_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:write") {
        return Err(ApiError::Forbidden);
    }
    let expense = ExpenseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    if !claims.is_admin() && expense.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }
    let updated =
        ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Submitted).await?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

/// POST /api/v1/expenses/:id/approve
pub async fn approve_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:approve") {
        return Err(ApiError::Forbidden);
    }
    let updated =
        ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Approved).await?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

/// POST /api/v1/expenses/:id/reject
pub async fn reject_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:approve") {
        return Err(ApiError::Forbidden);
    }
    let updated =
        ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Rejected).await?;
    Ok(Json(serde_json::json!({ "data": updated })))
}

/// POST /api/v1/expenses/:id/reimburse
pub async fn reimburse_expense(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:approve") {
        return Err(ApiError::Forbidden);
    }
    let updated =
        ExpenseRepo::transition(&state.db, &claims.org, &id, ExpenseStatus::Reimbursed).await?;
    Ok(Json(serde_json::json!({ "data": updated })))
}
