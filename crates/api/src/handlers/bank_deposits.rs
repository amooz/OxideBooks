use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateBankDeposit;
use oxidebooks_db::repos::BankDepositRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct DepositQuery {
    pub bank_account_id: Option<String>,
}

/// GET /api/v1/bank-deposits
pub async fn list_bank_deposits(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<DepositQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("banking:read") {
        return Err(ApiError::Forbidden);
    }
    let deposits =
        BankDepositRepo::list(&state.db, &claims.org, q.bank_account_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": deposits })))
}

/// GET /api/v1/bank-deposits/:id
pub async fn get_bank_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("banking:read") {
        return Err(ApiError::Forbidden);
    }
    let deposit = BankDepositRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": deposit })))
}

/// POST /api/v1/bank-deposits
pub async fn create_bank_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBankDeposit>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("banking:write") {
        return Err(ApiError::Forbidden);
    }
    let deposit = BankDepositRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": deposit })),
    ))
}

/// POST /api/v1/bank-deposits/:id/clear
pub async fn clear_bank_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("banking:write") {
        return Err(ApiError::Forbidden);
    }
    let deposit = BankDepositRepo::clear(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": deposit })))
}

/// DELETE /api/v1/bank-deposits/:id
pub async fn delete_bank_deposit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("banking:write") {
        return Err(ApiError::Forbidden);
    }
    BankDepositRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
