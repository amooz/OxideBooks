use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateLease, RecordLeasePayment, TerminateLease};
use oxidebooks_db::repos::LeaseRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct LeaseListQuery {
    pub status: Option<String>,
}

pub async fn list_leases(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<LeaseListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let leases = LeaseRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": leases })))
}

pub async fn get_lease(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let lease = LeaseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": lease })))
}

pub async fn create_lease(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateLease>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let lease = LeaseRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": lease })),
    ))
}

pub async fn get_lease_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let schedule = LeaseRepo::schedule(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

pub async fn record_lease_payment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<RecordLeasePayment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payment = LeaseRepo::record_payment(&state.db, &claims.org, &id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": payment })),
    ))
}

pub async fn list_lease_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let payments = LeaseRepo::list_payments(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": payments })))
}

pub async fn terminate_lease(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<TerminateLease>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let lease = LeaseRepo::terminate(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": lease })))
}
