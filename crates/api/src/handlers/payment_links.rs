use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreatePaymentLink;
use oxidebooks_db::repos::PaymentLinkRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_payment_links(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let links = PaymentLinkRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": links })))
}

pub async fn create_payment_link(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePaymentLink>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let link = PaymentLinkRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(link))))
}

pub async fn get_payment_link(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let link = PaymentLinkRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(link)))
}

pub async fn cancel_payment_link(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    PaymentLinkRepo::expire(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Public endpoint — no JWT required. Returns invoice summary for payment page.
pub async fn view_payment_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let link = PaymentLinkRepo::get_by_token(&state.db, &token).await?;
    if link.status != "active" {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(serde_json::json!(link)))
}
