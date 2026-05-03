use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateBatchPayment;
use oxidebooks_db::repos::BatchPaymentRepo;
use time::macros::format_description;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn parse_date(s: &str) -> Result<Date, ApiError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt)
        .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

/// GET /api/v1/batch-payments
pub async fn list_batch_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payments = BatchPaymentRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": payments })))
}

/// GET /api/v1/batch-payments/:id
pub async fn get_batch_payment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payment = BatchPaymentRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": payment })))
}

/// POST /api/v1/batch-payments
pub async fn create_batch_payment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBatchPayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.invoice_ids.is_empty() {
        return Err(ApiError::BadRequest("invoice_ids must not be empty".into()));
    }
    let payment_date = match &body.payment_date {
        Some(s) => parse_date(s)?,
        None => time::OffsetDateTime::now_utc().date(),
    };
    let (batch, succeeded, failed) =
        BatchPaymentRepo::create(&state.db, &claims.org, &claims.sub, body, payment_date).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "data": batch,
            "succeeded": succeeded,
            "failed": failed.iter().map(|(id, reason)| serde_json::json!({ "id": id, "reason": reason })).collect::<Vec<_>>(),
        })),
    ))
}
