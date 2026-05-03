use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateProject, UpdateProject};
use oxidebooks_db::repos::ProjectRepo;
use serde::Deserialize;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ProjectQuery {
    pub status: Option<String>,
}

pub async fn list_projects(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ProjectQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("projects:read") {
        return Err(ApiError::Forbidden);
    }
    let projects = ProjectRepo::list(&state.db, &claims.org, q.status.as_deref()).await?;
    Ok(Json(serde_json::json!({ "data": projects })))
}

pub async fn get_project(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("projects:read") {
        return Err(ApiError::Forbidden);
    }
    let project = ProjectRepo::get_by_id(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(project)))
}

pub async fn create_project(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateProject>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let project = ProjectRepo::create(&state.db, &claims.org, body).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(project))))
}

pub async fn update_project(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProject>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.is_at_least_accountant() {
        return Err(ApiError::Forbidden);
    }
    let project = ProjectRepo::update(&state.db, &claims.org, &id, body).await?;
    Ok(Json(serde_json::json!(project)))
}

pub async fn delete_project(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.is_admin() {
        return Err(ApiError::Forbidden);
    }
    ProjectRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn project_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("projects:read") {
        return Err(ApiError::Forbidden);
    }
    let summary = ProjectRepo::project_summary(&state.db, &claims.org, &id).await?;
    Ok(Json(serde_json::json!(summary)))
}
