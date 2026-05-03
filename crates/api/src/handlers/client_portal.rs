use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateClientPortalToken;
use oxidebooks_db::repos::{ClientPortalRepo, InvoiceRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn create_portal_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateClientPortalToken>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.expires_hours < 1 || body.expires_hours > 8760 {
        return Err(ApiError::BadRequest(
            "expires_hours must be between 1 and 8760".into(),
        ));
    }
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(body.expires_hours);
    let token =
        ClientPortalRepo::create_token(&state.db, &claims.org, &body.contact_id, expires_at)
            .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(token))))
}

/// Public (no JWT) — returns contact invoices for a valid portal token.
pub async fn portal_view(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;

    let filters = oxidebooks_core::models::InvoiceFilters {
        contact_id: Some(portal.contact_id.clone()),
        ..Default::default()
    };
    let page = oxidebooks_core::pagination::PageParams {
        limit: 100,
        after: None,
    };
    let (invoices, _) =
        InvoiceRepo::list(&state.db, &portal.organization_id, &page, &filters).await?;

    Ok(Json(serde_json::json!({
        "contact_id": portal.contact_id,
        "expires_at": portal.expires_at,
        "invoices": invoices,
    })))
}

/// Public (no JWT) — returns a single invoice for a valid portal token.
pub async fn portal_invoice(
    State(state): State<AppState>,
    Path((token, invoice_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let invoice = InvoiceRepo::get_by_id(&state.db, &portal.organization_id, &invoice_id).await?;

    if invoice.contact_id != portal.contact_id {
        return Err(ApiError::NotFound);
    }

    Ok(Json(serde_json::json!(invoice)))
}
