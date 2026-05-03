use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateInvoice, InvoiceFilters, UpdateInvoice};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::{AuditRepo, InvoiceRepo};
use serde::Deserialize;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct InvoiceQuery {
    #[serde(flatten)]
    pub page: PageParams,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub invoice_type: Option<String>,
    pub contact_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// GET /api/v1/invoices
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<InvoiceQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }

    let parse_date = |s: &str| {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
    };

    let filters = InvoiceFilters {
        status: q.status,
        invoice_type: q.invoice_type,
        contact_id: q.contact_id,
        from: q.from.as_deref().map(parse_date).transpose()?,
        to: q.to.as_deref().map(parse_date).transpose()?,
    };

    let (invoices, next_cursor) =
        InvoiceRepo::list(&state.db, &claims.org, &q.page, &filters).await?;
    Ok(Json(serde_json::json!({
        "data": invoices,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
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
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "create",
        "invoice",
        &invoice.id,
        None,
    )
    .await;
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
    info!(invoice_id = %id, org_id = %claims.org, status = %invoice.status, "invoice updated");
    let _ = AuditRepo::record(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        "update",
        "invoice",
        &id,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "data": invoice })))
}
