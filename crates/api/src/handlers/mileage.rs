use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateMileageTrip;
use oxidebooks_db::repos::MileageRepo;
use serde::Deserialize;

use crate::{error::ApiResult, middleware::Claims, state::AppState};

#[derive(Debug, Deserialize)]
pub struct MileageQuery {
    pub user_id: Option<String>,
}

pub async fn list_mileage_trips(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<MileageQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_filter = q.user_id.as_deref().or(Some(claims.sub.as_str()));
    let trips = MileageRepo::list(&state.db, &claims.org, user_filter).await?;
    Ok(Json(serde_json::json!({ "data": trips })))
}

pub async fn create_mileage_trip(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateMileageTrip>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let trip = MileageRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(trip))))
}

pub async fn delete_mileage_trip(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    MileageRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mileage_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<MileageQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let user_filter = q.user_id.as_deref().or(Some(claims.sub.as_str()));
    let summary = MileageRepo::summary(&state.db, &claims.org, user_filter).await?;
    Ok(Json(serde_json::json!(summary)))
}
