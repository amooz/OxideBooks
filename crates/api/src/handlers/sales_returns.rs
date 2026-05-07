use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApproveSalesReturn, CreateSalesReturn, ReceiveSalesReturn};
use oxidebooks_db::repos::SalesReturnRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ReturnQuery {
    pub status: Option<String>,
}

/// GET /api/v1/sales-returns
pub async fn list_sales_returns(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ReturnQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let returns = SalesReturnRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": returns })))
}

/// GET /api/v1/sales-returns/:id
pub async fn get_sales_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = SalesReturnRepo::get(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}

/// POST /api/v1/sales-returns
pub async fn create_sales_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSalesReturn>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = SalesReturnRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "data": r }))))
}

/// POST /api/v1/sales-returns/:id/approve
pub async fn approve_sales_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApproveSalesReturn>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = SalesReturnRepo::approve(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}

/// POST /api/v1/sales-returns/:id/receive
pub async fn receive_sales_return(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ReceiveSalesReturn>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let r = SalesReturnRepo::receive(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": r })))
}
