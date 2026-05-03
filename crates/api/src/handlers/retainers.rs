use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{ApplyRetainer, CreateRetainer, DepositRetainer};
use oxidebooks_db::repos::RetainerRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_retainers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let retainers = RetainerRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": retainers })))
}

pub async fn create_retainer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRetainer>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let retainer = RetainerRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(retainer))))
}

pub async fn deposit_retainer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<DepositRetainer>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let retainer = RetainerRepo::deposit(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(retainer)))
}

pub async fn apply_retainer(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<ApplyRetainer>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let retainer =
        RetainerRepo::apply(&state.db, &claims.org, &id, &body.invoice_id, body.amount).await?;
    Ok(Json(serde_json::json!(retainer)))
}

pub async fn list_retainer_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let txns = RetainerRepo::list_transactions(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": txns })))
}
