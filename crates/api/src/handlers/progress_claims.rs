use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProgressClaim, ReleaseRetainage};
use oxidebooks_db::repos::ProgressClaimRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/projects/:id/progress-claims
pub async fn list_progress_claims(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(project_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("projects:read") {
        return Err(ApiError::Forbidden);
    }
    let items = ProgressClaimRepo::list(&state.db, &claims.org, &project_id).await?;
    Ok(Json(serde_json::json!({ "data": items })))
}

/// POST /api/v1/projects/:id/progress-claims
pub async fn create_progress_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateProgressClaim>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let claim = ProgressClaimRepo::create(&state.db, &claims.org, &project_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": claim })),
    ))
}

/// POST /api/v1/progress-claims/:id/approve
pub async fn approve_progress_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(claim_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let claim = ProgressClaimRepo::approve(&state.db, &claims.org, &claim_id).await?;
    Ok(Json(serde_json::json!({ "data": claim })))
}

/// POST /api/v1/progress-claims/:id/invoice
pub async fn invoice_progress_claim(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(claim_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let claim = ProgressClaimRepo::convert_to_invoice(&state.db, &claims.org, &claim_id).await?;
    Ok(Json(serde_json::json!({ "data": claim })))
}

/// POST /api/v1/projects/:id/release-retainage
pub async fn release_retainage(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(project_id): Path<String>,
    Json(body): Json<ReleaseRetainage>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let claim =
        ProgressClaimRepo::release_retainage(&state.db, &claims.org, &project_id, body).await?;
    Ok(Json(serde_json::json!({ "data": claim })))
}

/// GET /api/v1/reports/project-billing
pub async fn project_billing_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let report = ProgressClaimRepo::project_billing_report(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": report })))
}
