use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_db::repos::GdprRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Deserialize)]
pub struct ContactIdQuery {
    pub contact_id: String,
}

pub async fn export_contact_data(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ContactIdQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let data = GdprRepo::export_contact_data(&state.db, &claims.org, &q.contact_id).await?;
    Ok(Json(serde_json::json!({ "data": data })))
}

pub async fn forget_contact(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ContactIdQuery>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    GdprRepo::forget_contact(&state.db, &claims.org, &q.contact_id, &claims.sub).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "contact anonymized" })),
    ))
}
