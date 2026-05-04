use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateRecurringBill, UpdateRecurringBill};
use oxidebooks_db::repos::RecurringBillRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RecurringBillQuery {
    pub active_only: Option<bool>,
}

/// GET /api/v1/recurring-bills
pub async fn list_recurring_bills(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<RecurringBillQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let active_only = q.active_only.unwrap_or(false);
    let items = RecurringBillRepo::list(&state.db, &claims.org, active_only).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/recurring-bills/:id
pub async fn get_recurring_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = RecurringBillRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/recurring-bills
pub async fn create_recurring_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRecurringBill>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.lines.is_empty() {
        return Err(ApiError::BadRequest("lines must not be empty".into()));
    }
    let item = RecurringBillRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// PATCH /api/v1/recurring-bills/:id
pub async fn update_recurring_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRecurringBill>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = RecurringBillRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// DELETE /api/v1/recurring-bills/:id
pub async fn delete_recurring_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<axum::http::StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    RecurringBillRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/recurring-bills/:id/generate
/// Generate a draft vendor bill from the template and advance next_due_date.
pub async fn generate_recurring_bill(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let bill = RecurringBillRepo::generate(&state.db, &claims.org, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": bill })),
    ))
}
