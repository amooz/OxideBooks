use axum::{
    extract::{Extension, Path, State},
    Json,
};
use oxidebooks_core::models::{CreateRevRecSchedule, RecognizeRevRec};
use oxidebooks_db::repos::RevRecRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn create_rev_rec_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<CreateRevRecSchedule>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let schedule = RevRecRepo::create_for_invoice(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": schedule })))
}

pub async fn get_rev_rec_schedule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let schedules = RevRecRepo::get_for_invoice(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": schedules })))
}

pub async fn list_rev_rec_schedules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let schedules = RevRecRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": schedules })))
}

pub async fn recognize_revenue(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<RecognizeRevRec>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let result = RevRecRepo::recognize(&state.db, &claims.org, body).await?;
    Ok(Json(serde_json::json!({ "data": result })))
}
