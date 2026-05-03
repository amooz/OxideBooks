use axum::{
    extract::{Extension, State},
    Json,
};
use oxidebooks_core::models::UpsertInvoiceTemplate;
use oxidebooks_db::repos::InvoiceTemplateRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/invoice-template
pub async fn get_invoice_template(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let template = InvoiceTemplateRepo::get(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": template })))
}

/// PUT /api/v1/invoice-template
pub async fn upsert_invoice_template(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpsertInvoiceTemplate>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if let Some(color) = &body.accent_color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(ApiError::BadRequest(
                "accent_color must be a 7-character hex string like #3b82f6".into(),
            ));
        }
    }
    let template = InvoiceTemplateRepo::upsert(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": template })))
}
