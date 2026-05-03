use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateBankAccount, ImportBankTransaction, MatchTransaction, UpdateBankAccount,
};
use oxidebooks_db::repos::BankRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct TxnQuery {
    pub status: Option<String>,
}

pub async fn list_bank_accounts(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bank:read") {
        return Err(ApiError::Forbidden);
    }
    let accounts = BankRepo::list_accounts(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": accounts })))
}

pub async fn get_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bank:read") {
        return Err(ApiError::Forbidden);
    }
    let account = BankRepo::get_account(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(account)))
}

pub async fn create_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBankAccount>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let account = BankRepo::create_account(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(account))))
}

pub async fn update_bank_account(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBankAccount>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let account = BankRepo::update_account(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(account)))
}

pub async fn list_bank_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Query(q): Query<TxnQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bank:read") {
        return Err(ApiError::Forbidden);
    }
    let txns =
        BankRepo::list_transactions(&state.db, &claims.org, &id, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": txns })))
}

pub async fn import_bank_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<Vec<ImportBankTransaction>>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let count = BankRepo::import_transactions(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "imported": count })))
}

pub async fn match_bank_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(txn_id): Path<String>,
    Json(body): Json<MatchTransaction>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let txn = BankRepo::match_transaction(&state.db, &claims.org, &txn_id, body).await?;
    Ok(Json(serde_json::json!(txn)))
}

pub async fn exclude_bank_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(txn_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let txn = BankRepo::exclude_transaction(&state.db, &claims.org, &txn_id).await?;
    Ok(Json(serde_json::json!(txn)))
}

pub async fn reconciliation_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("bank:read") {
        return Err(ApiError::Forbidden);
    }
    let summary = BankRepo::reconciliation_summary(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(summary)))
}
