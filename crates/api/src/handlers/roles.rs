use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{AssignPermission, CreateRole};
use oxidebooks_db::repos::{PermissionRepo, RoleRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/permissions
/// Lists all system permissions (requires roles:read).
pub async fn list_permissions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("roles:read") {
        return Err(ApiError::Forbidden);
    }
    let permissions = PermissionRepo::list(&state.db).await?;
    Ok(Json(serde_json::json!({ "data": permissions })))
}

/// GET /api/v1/roles
/// Lists all roles visible to the org (system + org-custom).
pub async fn list_roles(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("roles:read") {
        return Err(ApiError::Forbidden);
    }
    let roles = RoleRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": roles })))
}

/// POST /api/v1/roles
/// Creates a custom role for the org (requires roles:write).
pub async fn create_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateRole>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("roles:write") {
        return Err(ApiError::Forbidden);
    }
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("role name must not be empty".into()));
    }
    let role = RoleRepo::create(&state.db, &claims.org, body.name.trim()).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": role })),
    ))
}

/// POST /api/v1/roles/:id/permissions
/// Assigns a permission to a role (requires roles:write). Idempotent.
pub async fn assign_permission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(role_id): Path<String>,
    Json(body): Json<AssignPermission>,
) -> ApiResult<StatusCode> {
    if !claims.has("roles:write") {
        return Err(ApiError::Forbidden);
    }
    RoleRepo::assign_permission(&state.db, &claims.org, &role_id, &body.permission).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/roles/:role_id/permissions/:permission
/// Removes a permission from a role (requires roles:write).
pub async fn remove_permission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((role_id, permission)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    if !claims.has("roles:write") {
        return Err(ApiError::Forbidden);
    }
    RoleRepo::remove_permission(&state.db, &claims.org, &role_id, &permission).await?;
    Ok(StatusCode::NO_CONTENT)
}
