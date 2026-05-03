use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateAttachment;
use oxidebooks_db::repos::AttachmentRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_attachments(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((entity_type, entity_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let attachments =
        AttachmentRepo::list(&state.db, &claims.org, &entity_type, &entity_id).await?;
    Ok(Json(serde_json::json!({ "data": attachments })))
}

pub async fn create_attachment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((entity_type, entity_id)): Path<(String, String)>,
    Json(body): Json<CreateAttachment>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let attachment = AttachmentRepo::create(
        &state.db,
        &claims.org,
        &entity_type,
        &entity_id,
        Some(&claims.sub),
        body,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(attachment))))
}

pub async fn delete_attachment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((_entity_type, _entity_id, id)): Path<(String, String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let attachment = AttachmentRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(attachment)))
}
