use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateTaxRule, UpdateTaxRule};
use oxidebooks_db::repos::TaxRuleRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub applies_to: Option<String>,
    pub active_only: Option<bool>,
}

#[derive(Deserialize)]
pub struct SuggestQuery {
    pub country_code: String,
    pub region_code: Option<String>,
    #[serde(default = "default_applies_to")]
    pub applies_to: String,
}

fn default_applies_to() -> String {
    "sales".into()
}

/// GET /api/v1/tax-rules
pub async fn list_tax_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rules = TaxRuleRepo::list(
        &state.db,
        &claims.org,
        q.applies_to.as_deref(),
        q.active_only.unwrap_or(false),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": rules })))
}

/// GET /api/v1/tax-rules/:id
pub async fn get_tax_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let rule = TaxRuleRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": rule })))
}

/// POST /api/v1/tax-rules
pub async fn create_tax_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateTaxRule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = TaxRuleRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": rule })),
    ))
}

/// PATCH /api/v1/tax-rules/:id
pub async fn update_tax_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxRule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = TaxRuleRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": rule })))
}

/// DELETE /api/v1/tax-rules/:id
pub async fn delete_tax_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    TaxRuleRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/tax-rules/suggest
pub async fn suggest_tax_rate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SuggestQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let suggestion = TaxRuleRepo::suggest_for_contact(
        &state.db,
        &claims.org,
        &q.country_code,
        q.region_code.as_deref(),
        &q.applies_to,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": suggestion })))
}
