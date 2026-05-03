use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::UpsertExpensePolicy;
use oxidebooks_db::repos::ExpensePolicyRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_expense_policies(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let policies = ExpensePolicyRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": policies })))
}

pub async fn upsert_expense_policy(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(category): Path<String>,
    Json(body): Json<UpsertExpensePolicy>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if body.max_amount <= 0 {
        return Err(ApiError::BadRequest("max_amount must be positive".into()));
    }
    let policy = ExpensePolicyRepo::upsert(&state.db, &claims.org, &category, body).await?;
    Ok(Json(serde_json::json!(policy)))
}

pub async fn delete_expense_policy(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(category): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ExpensePolicyRepo::delete(&state.db, &claims.org, &category).await?;
    Ok(StatusCode::NO_CONTENT)
}
