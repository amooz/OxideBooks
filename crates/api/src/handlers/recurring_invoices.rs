use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateRecurringInvoice, UpdateRecurringInvoice};
use oxidebooks_db::repos::RecurringInvoiceRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RecurringInvoiceQuery {
    pub active_only: Option<bool>,
}

/// GET /api/v1/recurring-invoices
pub async fn list_recurring_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<RecurringInvoiceQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let active_only = q.active_only.unwrap_or(false);
    let items = RecurringInvoiceRepo::list(&state.db, &claims.org, active_only).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// GET /api/v1/recurring-invoices/:id
pub async fn get_recurring_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = RecurringInvoiceRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// POST /api/v1/recurring-invoices
pub async fn create_recurring_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRecurringInvoice>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.lines.is_empty() {
        return Err(ApiError::BadRequest("lines must not be empty".into()));
    }
    let item = RecurringInvoiceRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": item })),
    ))
}

/// PATCH /api/v1/recurring-invoices/:id
pub async fn update_recurring_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRecurringInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let item = RecurringInvoiceRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": item })))
}

/// DELETE /api/v1/recurring-invoices/:id
pub async fn delete_recurring_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<axum::http::StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    RecurringInvoiceRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/recurring-invoices/:id/generate
/// Generate a draft invoice from the template and advance next_due_date.
pub async fn generate_recurring_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let invoice = RecurringInvoiceRepo::generate(&state.db, &claims.org, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}
