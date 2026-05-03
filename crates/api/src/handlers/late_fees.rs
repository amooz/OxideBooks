use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{InvoiceStatus, UpsertLateFeeRule};
use oxidebooks_db::repos::{InvoiceRepo, LateFeeRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn get_late_fee_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let rule = LateFeeRepo::get_rule(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rule })))
}

pub async fn upsert_late_fee_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpsertLateFeeRule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if body.fee_type != "flat" && body.fee_type != "percent" {
        return Err(ApiError::BadRequest(
            "fee_type must be 'flat' or 'percent'".into(),
        ));
    }
    let rule = LateFeeRepo::upsert_rule(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!(rule)))
}

pub async fn apply_late_fee(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }

    let rule = LateFeeRepo::get_rule(&state.db, &claims.org)
        .await?
        .ok_or_else(|| ApiError::BadRequest("no late fee rule configured".into()))?;

    let invoice = InvoiceRepo::get_by_id(&state.db, &claims.org, &invoice_id).await?;

    if invoice.status == InvoiceStatus::Paid
        || invoice.status == InvoiceStatus::Voided
        || invoice.status == InvoiceStatus::Draft
    {
        return Err(ApiError::BadRequest(
            "invoice is not eligible for a late fee".into(),
        ));
    }

    let today = time::OffsetDateTime::now_utc().date();
    let grace_end = invoice
        .due_date
        .checked_add(time::Duration::days(rule.grace_days as i64))
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("date overflow")))?;

    if today <= grace_end {
        return Err(ApiError::BadRequest(
            "invoice is within grace period".into(),
        ));
    }

    let invoice_total = invoice.total();
    let fee_amount = match rule.fee_type.as_str() {
        "flat" => rule.amount,
        "percent" => invoice_total * rule.amount / 10_000,
        _ => return Err(ApiError::Internal(anyhow::anyhow!("unknown fee type"))),
    };

    let fee = LateFeeRepo::record_fee(&state.db, &claims.org, &invoice_id, fee_amount).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(fee))))
}

pub async fn list_late_fees(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let fees = LateFeeRepo::list_for_invoice(&state.db, &claims.org, &invoice_id).await?;
    Ok(Json(serde_json::json!({ "data": fees })))
}
