use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateBankRule;
use oxidebooks_db::repos::BankRuleRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/bank-rules
pub async fn list_bank_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rules = BankRuleRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rules })))
}

/// POST /api/v1/bank-rules
pub async fn create_bank_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBankRule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let valid_fields = ["description", "amount"];
    if !valid_fields.contains(&body.match_field.as_str()) {
        return Err(ApiError::BadRequest(
            "match_field must be 'description' or 'amount'".into(),
        ));
    }
    let valid_types = ["contains", "equals", "gt", "lt"];
    if !valid_types.contains(&body.match_type.as_str()) {
        return Err(ApiError::BadRequest(
            "match_type must be 'contains', 'equals', 'gt', or 'lt'".into(),
        ));
    }
    let rule = BankRuleRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rule })),
    ))
}

/// DELETE /api/v1/bank-rules/:id
pub async fn delete_bank_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    BankRuleRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/bank-accounts/:id/apply-rules
pub async fn apply_bank_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let (matched, skipped) = BankRuleRepo::apply_rules(&state.db, &claims.org, &id).await?;
    Ok(Json(
        serde_json::json!({ "matched": matched, "skipped": skipped }),
    ))
}
