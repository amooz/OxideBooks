use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateBudget, UpdateBudget, UpsertBudgetLine};
use oxidebooks_db::repos::BudgetRepo;
use serde::Deserialize;
use time::macros::format_description;
use time::Date;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

fn parse_date(s: &str) -> Result<Date, ApiError> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, fmt)
        .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

/// GET /api/v1/budgets
pub async fn list_budgets(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let budgets = BudgetRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": budgets })))
}

/// GET /api/v1/budgets/:id
pub async fn get_budget(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let budget = BudgetRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let lines = BudgetRepo::list_lines(&state.db, &id).await?;
    Ok(Json(
        serde_json::json!({ "data": { "budget": budget, "lines": lines } }),
    ))
}

/// POST /api/v1/budgets
pub async fn create_budget(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBudget>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let budget = BudgetRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": budget })),
    ))
}

/// PATCH /api/v1/budgets/:id
pub async fn update_budget(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBudget>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let budget = BudgetRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": budget })))
}

/// DELETE /api/v1/budgets/:id
pub async fn delete_budget(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    BudgetRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/v1/budgets/:id/lines
pub async fn upsert_budget_lines(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<Vec<UpsertBudgetLine>>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    // Verify budget belongs to org
    BudgetRepo::get_by_id(&state.db, &claims.org, &id).await?;
    let lines = BudgetRepo::upsert_lines(&state.db, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": lines })))
}

#[derive(Deserialize)]
pub struct BudgetVsActualQuery {
    pub budget_id: String,
    pub from: String,
    pub to: String,
}

/// GET /api/v1/reports/budget-vs-actual?budget_id=&from=&to=
pub async fn budget_vs_actual(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<BudgetVsActualQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let from = parse_date(&q.from)?;
    let to = parse_date(&q.to)?;
    if from > to {
        return Err(ApiError::BadRequest(
            "'from' must be on or before 'to'".into(),
        ));
    }
    let report =
        BudgetRepo::budget_vs_actual(&state.db, &claims.org, &q.budget_id, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
