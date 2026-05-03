use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateApprovalRule, UpdateApprovalRule};
use oxidebooks_db::repos::ApprovalRuleRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RuleQuery {
    pub entity_type: Option<String>,
}

/// GET /api/v1/approval-rules
pub async fn list_approval_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<RuleQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rules = ApprovalRuleRepo::list(&state.db, &claims.org, q.entity_type.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": rules })))
}

/// GET /api/v1/approval-rules/:id
pub async fn get_approval_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = ApprovalRuleRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": rule })))
}

/// POST /api/v1/approval-rules
pub async fn create_approval_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateApprovalRule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = ApprovalRuleRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rule })),
    ))
}

/// PATCH /api/v1/approval-rules/:id
pub async fn update_approval_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateApprovalRule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = ApprovalRuleRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": rule })))
}

/// DELETE /api/v1/approval-rules/:id
pub async fn delete_approval_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ApprovalRuleRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
