use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePortalAutopay, CreatePortalPaymentMethod};
use oxidebooks_db::repos::{ClientPortalRepo, PortalPaymentMethodRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

// ── Portal-token-authenticated endpoints ──────────────────────────────────────

pub async fn portal_list_payment_methods(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let methods =
        PortalPaymentMethodRepo::list(&state.db, &portal.organization_id, &portal.contact_id)
            .await?;
    Ok(Json(serde_json::json!({ "data": methods })))
}

pub async fn portal_add_payment_method(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(body): Json<CreatePortalPaymentMethod>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let method =
        PortalPaymentMethodRepo::add(&state.db, &portal.organization_id, &portal.contact_id, body)
            .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": method })),
    ))
}

pub async fn portal_delete_payment_method(
    State(state): State<AppState>,
    Path((token, pm_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    PortalPaymentMethodRepo::delete(
        &state.db,
        &portal.organization_id,
        &portal.contact_id,
        &pm_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn portal_set_default_payment_method(
    State(state): State<AppState>,
    Path((token, pm_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let method = PortalPaymentMethodRepo::set_default(
        &state.db,
        &portal.organization_id,
        &portal.contact_id,
        &pm_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": method })))
}

pub async fn portal_get_autopay(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let enrollment = PortalPaymentMethodRepo::get_autopay(
        &state.db,
        &portal.organization_id,
        &portal.contact_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": enrollment })))
}

pub async fn portal_enroll_autopay(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(body): Json<CreatePortalAutopay>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    let enrollment = PortalPaymentMethodRepo::enroll_autopay(
        &state.db,
        &portal.organization_id,
        &portal.contact_id,
        body,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": enrollment })),
    ))
}

pub async fn portal_cancel_autopay(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<StatusCode> {
    let portal = ClientPortalRepo::get_by_token(&state.db, &token).await?;
    PortalPaymentMethodRepo::cancel_autopay(&state.db, &portal.organization_id, &portal.contact_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── JWT-authenticated admin endpoints ─────────────────────────────────────────

pub async fn admin_list_payment_methods(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(contact_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let methods = PortalPaymentMethodRepo::list(&state.db, &claims.org, &contact_id).await?;
    Ok(Json(serde_json::json!({ "data": methods })))
}

pub async fn admin_get_autopay(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(contact_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let enrollment =
        PortalPaymentMethodRepo::get_autopay(&state.db, &claims.org, &contact_id).await?;
    Ok(Json(serde_json::json!({ "data": enrollment })))
}

pub async fn admin_cancel_autopay(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(contact_id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    PortalPaymentMethodRepo::cancel_autopay(&state.db, &claims.org, &contact_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
