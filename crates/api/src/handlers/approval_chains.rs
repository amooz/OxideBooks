use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use oxidebooks_core::models::{CreateApprovalChain, RecordApprovalDecision, SubmitApprovalRequest};
use oxidebooks_db::repos::ApprovalChainRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
}

pub async fn create_chain(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateApprovalChain>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let chain = ApprovalChainRepo::create_chain(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": chain })))
}

pub async fn list_chains(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let chains = ApprovalChainRepo::list_chains(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": chains })))
}

pub async fn get_chain(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let chain = ApprovalChainRepo::get_chain(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": chain })))
}

pub async fn submit_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<SubmitApprovalRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let request =
        ApprovalChainRepo::submit_request(&state.db, &claims.org, &claims.sub, body).await?;
    Ok(Json(serde_json::json!({ "data": request })))
}

pub async fn list_requests(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<StatusQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let requests =
        ApprovalChainRepo::list_requests(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": requests })))
}

pub async fn get_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let request = ApprovalChainRepo::get_request(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": request })))
}

pub async fn approve_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<RecordApprovalDecision>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut input = body;
    input.decision = "approved".to_string();
    let request =
        ApprovalChainRepo::decide(&state.db, &claims.org, &id, &claims.sub, input).await?;
    Ok(Json(serde_json::json!({ "data": request })))
}

pub async fn reject_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<RecordApprovalDecision>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let mut input = body;
    input.decision = "rejected".to_string();
    let request =
        ApprovalChainRepo::decide(&state.db, &claims.org, &id, &claims.sub, input).await?;
    Ok(Json(serde_json::json!({ "data": request })))
}
