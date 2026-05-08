use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{
    CreateSubscription, CreateSubscriptionPlan, UpdateSubscription, UpdateSubscriptionPlan,
};
use oxidebooks_db::repos::{BillingRunResult, SubscriptionRepo};
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

/// POST /api/v1/subscriptions/:id/bill
/// Creates a draft invoice for the current billing period and advances the period.
pub async fn bill_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let invoice = SubscriptionRepo::bill(&state.db, &claims.org, &id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": invoice })),
    ))
}

#[derive(Deserialize)]
pub struct MrrQuery {
    pub as_of: Option<String>,
}

/// GET /api/v1/reports/subscription-mrr
pub async fn subscription_mrr(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<MrrQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let as_of = if let Some(s) = q.as_of.as_deref() {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))?
    } else {
        time::OffsetDateTime::now_utc().date()
    };
    let snapshot = SubscriptionRepo::mrr_snapshot(&state.db, &claims.org, as_of).await?;
    Ok(Json(serde_json::json!({ "data": snapshot })))
}

#[derive(Deserialize)]
pub struct ChurnQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

/// GET /api/v1/reports/subscription-churn
pub async fn subscription_churn(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ChurnQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    let today = time::OffsetDateTime::now_utc().date();
    let to = if let Some(s) = q.to.as_deref() {
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))?
    } else {
        today
    };
    let from = if let Some(s) = q.from.as_deref() {
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))?
    } else {
        // default: last 30 days
        to - time::Duration::days(30)
    };
    let report = SubscriptionRepo::churn_report(&state.db, &claims.org, from, to).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}

#[derive(Deserialize)]
pub struct BillingRunQuery {
    pub as_of: Option<String>,
}

/// POST /api/v1/subscriptions/billing-run
/// Generate invoices for all active subscriptions due on or before `as_of` (defaults to today).
pub async fn billing_run(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<BillingRunQuery>,
) -> ApiResult<axum::Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let as_of = if let Some(s) = q.as_of.as_deref() {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        time::Date::parse(s, fmt)
            .map_err(|_| ApiError::BadRequest(format!("invalid date '{s}'; expected YYYY-MM-DD")))?
    } else {
        time::OffsetDateTime::now_utc().date()
    };
    let result: BillingRunResult =
        SubscriptionRepo::bill_due(&state.db, &claims.org, as_of).await?;
    Ok(axum::Json(serde_json::json!({ "data": result })))
}
