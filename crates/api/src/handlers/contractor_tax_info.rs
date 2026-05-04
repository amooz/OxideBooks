use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateContractorTaxInfo, UpdateContractorTaxInfo};
use oxidebooks_db::repos::ContractorTaxInfoRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct PaymentsQuery {
    pub year: i32,
    /// Reporting threshold in minor units; defaults to $600.00 (60000 cents)
    #[serde(default = "default_threshold")]
    pub threshold: i64,
}

fn default_threshold() -> i64 {
    60_000
}

/// GET /api/v1/contractor-tax-info
pub async fn list_contractor_tax_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let infos = ContractorTaxInfoRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": infos })))
}

/// GET /api/v1/contractor-tax-info/:id
pub async fn get_contractor_tax_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let info = ContractorTaxInfoRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": info })))
}

/// GET /api/v1/contacts/:id/contractor-tax-info
pub async fn get_contact_contractor_tax_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(contact_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let info = ContractorTaxInfoRepo::get_by_contact(&state.db, &claims.org, &contact_id).await?;
    Ok(Json(serde_json::json!({ "data": info })))
}

/// POST /api/v1/contractor-tax-info
pub async fn create_contractor_tax_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateContractorTaxInfo>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let info = ContractorTaxInfoRepo::create(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": info })),
    ))
}

/// PATCH /api/v1/contractor-tax-info/:id
pub async fn update_contractor_tax_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateContractorTaxInfo>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let info = ContractorTaxInfoRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": info })))
}

/// GET /api/v1/reports/1099-payments?year=2025&threshold=60000
pub async fn report_1099_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PaymentsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let summaries = ContractorTaxInfoRepo::list_1099_payments(
        &state.db,
        &claims.org,
        params.year,
        params.threshold,
    )
    .await?;
    Ok(Json(serde_json::json!({ "data": summaries })))
}
