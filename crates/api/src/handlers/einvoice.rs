use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{InboundEInvoice, SendEInvoice};
use oxidebooks_db::repos::EInvoiceRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn send_einvoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<SendEInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let transmission = EInvoiceRepo::send(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": transmission })))
}

pub async fn einvoice_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let transmissions = EInvoiceRepo::get_status(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": transmissions })))
}

pub async fn receive_einvoice(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<InboundEInvoice>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let result = EInvoiceRepo::receive(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": result })))
}
