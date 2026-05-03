use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateApiKey;
use oxidebooks_db::repos::ApiKeyRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let keys = ApiKeyRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": keys })))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateApiKey>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let created = ApiKeyRepo::create(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(created))))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ApiKeyRepo::revoke(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
