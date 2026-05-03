use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_db::repos::NotificationRepo;
use serde::Deserialize;

use crate::{error::ApiResult, middleware::Claims, state::AppState};

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub unread_only: Option<bool>,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<NotificationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let unread_only = q.unread_only.unwrap_or(false);
    let notifications =
        NotificationRepo::list(&state.db, &claims.org, &claims.sub, unread_only).await?;
    Ok(Json(serde_json::json!({ "data": notifications })))
}

pub async fn mark_notification_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    NotificationRepo::mark_read(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_all_notifications_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let count = NotificationRepo::mark_all_read(&state.db, &claims.org, &claims.sub).await?;
    Ok(Json(serde_json::json!({ "updated": count })))
}
