use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateSubscription, CreateSubscriptionPlan, UpdateSubscription, UpdateSubscriptionPlan,
};
use oxidebooks_db::repos::SubscriptionRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SubQuery {
    pub status: Option<String>,
    pub contact_id: Option<String>,
}

// ── Plans ─────────────────────────────────────────────────────────────────────

pub async fn list_plans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let plans = SubscriptionRepo::list_plans(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": plans })))
}

pub async fn get_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let plan = SubscriptionRepo::get_plan(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(plan)))
}

pub async fn create_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSubscriptionPlan>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let plan = SubscriptionRepo::create_plan(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(plan))))
}

pub async fn update_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscriptionPlan>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let plan = SubscriptionRepo::update_plan(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(plan)))
}

// ── Subscriptions ─────────────────────────────────────────────────────────────

pub async fn list_subscriptions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<SubQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let subs = SubscriptionRepo::list(
        &state.db,
        &claims.org,
        q.status.as_deref(),
        q.contact_id.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": subs })))
}

pub async fn get_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("invoices:read") {
        return Err(ApiError::Forbidden);
    }
    let sub = SubscriptionRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(sub)))
}

pub async fn create_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSubscription>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let sub = SubscriptionRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(sub))))
}

pub async fn update_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubscription>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let sub = SubscriptionRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(sub)))
}

pub async fn cancel_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let sub = SubscriptionRepo::cancel(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(sub)))
}

pub async fn renew_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let sub = SubscriptionRepo::renew(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(sub)))
}
