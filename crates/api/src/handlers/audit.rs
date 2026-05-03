use axum::{
    extract::{Extension, Query, State},
    Json,
};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::AuditRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(flatten)]
    pub page: PageParams,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
}

/// GET /api/v1/audit-log
pub async fn list_audit_log(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AuditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("audit:read") {
        return Err(ApiError::Forbidden);
    }
    let (events, next_cursor) = AuditRepo::list(
        &state.db,
        &claims.org,
        q.resource_type.as_deref(),
        q.resource_id.as_deref(),
        &q.page,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "data": events,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}
