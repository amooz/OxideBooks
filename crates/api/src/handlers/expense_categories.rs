use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateExpenseCategory, UpdateExpenseCategory};
use oxidebooks_db::repos::ExpenseCategoryRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/expense-categories
pub async fn list_expense_categories(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let cats = ExpenseCategoryRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": cats })))
}

/// GET /api/v1/expense-categories/:id
pub async fn get_expense_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("expenses:read") {
        return Err(ApiError::Forbidden);
    }
    let cat = ExpenseCategoryRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": cat })))
}

/// POST /api/v1/expense-categories
pub async fn create_expense_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateExpenseCategory>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let cat = ExpenseCategoryRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": cat })),
    ))
}

/// PATCH /api/v1/expense-categories/:id
pub async fn update_expense_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateExpenseCategory>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let cat = ExpenseCategoryRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": cat })))
}

/// DELETE /api/v1/expense-categories/:id
pub async fn delete_expense_category(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    ExpenseCategoryRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
