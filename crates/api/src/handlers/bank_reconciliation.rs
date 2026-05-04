use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateBankReconciliationStatement;
use oxidebooks_db::repos::BankReconciliationRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ReconciliationQuery {
    pub bank_account_id: Option<String>,
}

/// GET /api/v1/bank-reconciliation-statements
pub async fn list_reconciliation_statements(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ReconciliationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let stmts =
        BankReconciliationRepo::list(&state.db, &claims.org, q.bank_account_id.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": stmts })))
}

/// GET /api/v1/bank-reconciliation-statements/:id
pub async fn get_reconciliation_statement(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let stmt = BankReconciliationRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": stmt })))
}

/// POST /api/v1/bank-reconciliation-statements
pub async fn create_reconciliation_statement(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBankReconciliationStatement>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let stmt = BankReconciliationRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": stmt })),
    ))
}

/// DELETE /api/v1/bank-reconciliation-statements/:id
pub async fn delete_reconciliation_statement(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    BankReconciliationRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": null })))
}
