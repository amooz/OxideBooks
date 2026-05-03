use axum::{
    extract::{Extension, Query, State},
    Json,
};
use oxidebooks_db::repos::FxRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct FxQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn fx_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<FxQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("reports:read") {
        return Err(ApiError::Forbidden);
    }
    let rows =
        FxRepo::fx_summary(&state.db, &claims.org, q.from.as_deref(), q.to.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}
