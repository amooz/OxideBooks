use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_db::repos::SessionRepo;

use crate::{error::ApiResult, middleware::Claims, state::AppState};

pub async fn list_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let sessions = SessionRepo::list(&state.db, &claims.sub).await?;
    Ok(Json(serde_json::json!({ "data": sessions })))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    SessionRepo::revoke(&state.db, &claims.sub, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_all_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let revoked = SessionRepo::revoke_all_except(&state.db, &claims.sub, &claims.jti).await?;
    Ok(Json(serde_json::json!({ "revoked": revoked })))
}
