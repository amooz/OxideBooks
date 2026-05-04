use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateSalesCommission, PayCommission};
use oxidebooks_db::repos::CommissionRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/commissions
pub async fn list_commissions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let commissions = CommissionRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": commissions })))
}

/// GET /api/v1/invoices/:id/commissions
pub async fn list_invoice_commissions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let commissions = CommissionRepo::list_for_invoice(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": commissions })))
}

/// GET /api/v1/commissions/:id
pub async fn get_commission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let commission = CommissionRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": commission })))
}

/// POST /api/v1/invoices/:id/commissions
pub async fn create_commission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
    Json(mut body): Json<CreateSalesCommission>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    body.invoice_id = invoice_id;
    let commission = CommissionRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": commission })),
    ))
}

/// POST /api/v1/commissions/:id/approve
pub async fn approve_commission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let commission = CommissionRepo::approve(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": commission })))
}

/// POST /api/v1/commissions/:id/pay
pub async fn pay_commission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<PayCommission>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let commission = CommissionRepo::pay(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": commission })))
}

/// POST /api/v1/commissions/:id/void
pub async fn void_commission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let commission = CommissionRepo::void(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": commission })))
}
