use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateInvoice, UpdateInvoice};
use oxidebooks_db::repos::InvoiceRepo;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/invoices
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let invoices = InvoiceRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": invoices })))
}

/// GET /api/v1/invoices/:id
pub async fn get_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": invoice })))
}

/// POST /api/v1/invoices
pub async fn create_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateInvoice>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}

/// PATCH /api/v1/invoices/:id
pub async fn update_invoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    let invoice = InvoiceRepo::update(&state.db, &claims.org, &id, body).await?;
    info!(
        invoice_id = %id,
        org_id = %claims.org,
        status = %invoice.status,
        "📋 invoice updated"
    );
    Ok(Json(serde_json::json!({ "data": invoice })))
}
