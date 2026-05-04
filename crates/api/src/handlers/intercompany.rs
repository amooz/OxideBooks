use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateIntercompanyLink, CreateIntercompanyTransaction};
use oxidebooks_db::repos::IntercompanyRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/intercompany/links
pub async fn list_links(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let links = IntercompanyRepo::list_links(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": links })))
}

/// POST /api/v1/intercompany/links
pub async fn create_link(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateIntercompanyLink>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let link = IntercompanyRepo::create_link(&state.db, &claims.org, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": link })),
    ))
}

/// DELETE /api/v1/intercompany/links/:id
pub async fn delete_link(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    IntercompanyRepo::delete_link(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/intercompany/transactions
pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    let txns = IntercompanyRepo::list_transactions(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": txns })))
}

/// POST /api/v1/intercompany/transactions
pub async fn create_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateIntercompanyTransaction>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    if body.amount <= 0 {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }
    let txn =
        IntercompanyRepo::create_transaction(&state.db, &claims.org, &claims.sub, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": txn })),
    ))
}
