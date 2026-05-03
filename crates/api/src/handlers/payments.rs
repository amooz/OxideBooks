use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePayment, VALID_METHODS};
use oxidebooks_db::repos::PaymentRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// POST /api/v1/invoices/:id/payments
pub async fn create_payment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
    Json(body): Json<CreatePayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("invoices:write") {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    if !VALID_METHODS.contains(&body.method.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "invalid method '{}'; valid: {}",
            body.method,
            VALID_METHODS.join(", ")
        )));
    }
    let payment = PaymentRepo::create(&state.db, &claims.org, &invoice_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": payment })),
    ))
}

/// GET /api/v1/invoices/:id/payments
pub async fn list_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let payments = PaymentRepo::list_by_invoice(&state.db, &claims.org, &invoice_id).await?;
    Ok(Json(serde_json::json!({ "data": payments })))
}
