use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateVendorPortalToken;
use oxidebooks_db::repos::{BillRepo, VendorPortalRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// POST /api/v1/vendor-portal/tokens
pub async fn create_vendor_portal_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateVendorPortalToken>,
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
        VendorPortalRepo::create_token(&state.db, &claims.org, &body.contact_id, expires_at)
            .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(token))))
}

/// DELETE /api/v1/vendor-portal/tokens/:contact_id — revoke token for a contact
pub async fn revoke_vendor_portal_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(contact_id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    VendorPortalRepo::revoke(&state.db, &claims.org, &contact_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Public (no JWT) — returns vendor's bills for a valid portal token.
pub async fn vendor_portal_view(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = VendorPortalRepo::get_by_token(&state.db, &token).await?;
    let bills =
        BillRepo::list_for_contact(&state.db, &portal.organization_id, &portal.contact_id).await?;
    Ok(Json(serde_json::json!({
        "contact_id": portal.contact_id,
        "expires_at": portal.expires_at,
        "bills": bills,
    })))
}

/// Public (no JWT) — returns a single bill for a valid portal token.
pub async fn vendor_portal_bill(
    State(state): State<AppState>,
    Path((token, bill_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = VendorPortalRepo::get_by_token(&state.db, &token).await?;
    let bill = BillRepo::get_by_id(&state.db, &portal.organization_id, &bill_id).await?;
    if bill.contact_id.as_deref() != Some(portal.contact_id.as_str()) {
        return Err(ApiError::NotFound);
    }
    Ok(Json(serde_json::json!(bill)))
}
