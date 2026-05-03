use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreatePaymentPlan, PayInstallment};
use oxidebooks_db::repos::PaymentPlanRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub invoice_id: Option<String>,
}

pub async fn list_payment_plans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<PlanQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let plans = PaymentPlanRepo::list(&state.db, &claims.org, q.invoice_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": plans })))
}

pub async fn get_payment_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let plan = PaymentPlanRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(plan)))
}

pub async fn create_payment_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreatePaymentPlan>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let plan = PaymentPlanRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(plan))))
}

pub async fn pay_installment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((id, inst_id)): Path<(String, String)>,
    Json(body): Json<PayInstallment>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let plan =
        PaymentPlanRepo::pay_installment(&state.db, &claims.org, &id, &inst_id, body).await?;
    Ok(Json(serde_json::json!(plan)))
}

pub async fn cancel_payment_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let plan = PaymentPlanRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(plan)))
}
