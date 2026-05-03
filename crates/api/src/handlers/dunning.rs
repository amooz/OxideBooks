use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateDunningRule;
use oxidebooks_db::repos::DunningRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_dunning_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rules = DunningRepo::list_rules(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": rules })))
}

pub async fn upsert_dunning_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDunningRule>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let rule = DunningRepo::create_rule(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(rule))))
}

pub async fn delete_dunning_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    DunningRepo::delete_rule(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_overdue_invoices(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let overdue = DunningRepo::overdue_invoices(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": overdue })))
}

pub async fn send_reminder(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let to = body["to"]
        .as_str()
        .ok_or_else(|| ApiError::BadRequest("missing 'to' field".to_string()))?;
    let level = body["level"].as_i64().unwrap_or(1) as i32;
    let reminder = DunningRepo::record_reminder(&state.db, &invoice_id, None, to, level).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(reminder))))
}

pub async fn list_invoice_reminders(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(invoice_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let reminders = DunningRepo::list_reminders(&state.db, &invoice_id).await?;
    Ok(Json(serde_json::json!({ "data": reminders })))
}
