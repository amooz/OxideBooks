use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProjectPhase, UpdateProjectPhase};
use oxidebooks_db::repos::ProjectPhaseRepo;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/projects/:id/phases
pub async fn list_phases(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(project_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let phases = ProjectPhaseRepo::list_for_project(&state.db, &claims.org, &project_id).await?;
    Ok(Json(serde_json::json!({ "data": phases })))
}

/// GET /api/v1/project-phases/:id
pub async fn get_phase(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let phase = ProjectPhaseRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!({ "data": phase })))
}

/// POST /api/v1/projects/:id/phases
pub async fn create_phase(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateProjectPhase>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let phase = ProjectPhaseRepo::create(&state.db, &claims.org, &project_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": phase })),
    ))
}

/// PATCH /api/v1/project-phases/:id
pub async fn update_phase(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProjectPhase>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let phase = ProjectPhaseRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!({ "data": phase })))
}

/// DELETE /api/v1/project-phases/:id
pub async fn delete_phase(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ProjectPhaseRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
