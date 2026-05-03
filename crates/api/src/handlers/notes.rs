use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::CreateNote;
use oxidebooks_db::repos::NoteRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn list_notes(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((entity_type, entity_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let notes = NoteRepo::list(&state.db, &claims.org, &entity_type, &entity_id).await?;
    Ok(Json(serde_json::json!({ "data": notes })))
}

pub async fn create_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((entity_type, entity_id)): Path<(String, String)>,
    Json(body): Json<CreateNote>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let note = NoteRepo::create(
        &state.db,
        &claims.org,
        Some(&claims.sub),
        &entity_type,
        &entity_id,
        body,
        false,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(note))))
}

pub async fn delete_note(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((_entity_type, _entity_id, id)): Path<(String, String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    NoteRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
