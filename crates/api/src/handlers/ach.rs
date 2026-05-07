use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{CollectAch, GenerateNachaRequest, PayBillAch};
use oxidebooks_db::repos::AchRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn collect_ach(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CollectAch>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payment = AchRepo::collect_ach(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": payment })))
}

pub async fn pay_bill_ach(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<PayBillAch>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let payment = AchRepo::pay_bill_ach(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": payment })))
}

pub async fn generate_nacha(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<GenerateNachaRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let file = AchRepo::generate_nacha(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": file })))
}

pub async fn list_ach_payments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let payments = AchRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": payments })))
}
