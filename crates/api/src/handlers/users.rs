use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::pagination::PageParams;
use oxidebooks_db::repos::users::{CreateUser, UpdateUser, UserRepo};
use serde::Deserialize;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

/// GET /api/v1/users
pub async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("users:read") {
        return Err(ApiError::Forbidden);
    }
    let (users, next_cursor) = UserRepo::list(&state.db, &claims.org, &page).await?;
    Ok(Json(serde_json::json!({
        "data": users,
        "pagination": { "has_next": next_cursor.is_some(), "next_cursor": next_cursor }
    })))
}

/// GET /api/v1/users/:id
pub async fn get_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("users:read") {
        return Err(ApiError::Forbidden);
    }
    let user = UserRepo::get_by_id(&state.db, &id).await?;
    // Ensure the user belongs to the caller's org.
    if user.organization_id != claims.org {
        return Err(ApiError::NotFound);
    }
    Ok(Json(serde_json::json!({ "data": user })))
}

#[derive(Debug, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub name: String,
    pub role: String,
}

/// POST /api/v1/users
/// Invites a new user to the organization with an empty password hash.
/// The invited user authenticates via SSO or sets a password via the password-reset flow.
pub async fn invite_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<InviteUserRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }

    let valid_roles = ["viewer", "accountant", "admin", "owner"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "role must be one of: {}",
            valid_roles.join(", ")
        )));
    }

    let user = UserRepo::create(
        &state.db,
        CreateUser {
            organization_id: claims.org.clone(),
            email: body.email,
            password_hash: String::new(),
            name: body.name,
            role: body.role,
        },
    )
    .await?;

    info!(
        user_id = %user.id,
        org_id = %claims.org,
        invited_by = %claims.sub,
        "👤 user invited"
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": user })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

/// PATCH /api/v1/users/:id
pub async fn update_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }

    if let Some(ref role) = body.role {
        let valid_roles = ["viewer", "accountant", "admin", "owner"];
        if !valid_roles.contains(&role.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "role must be one of: {}",
                valid_roles.join(", ")
            )));
        }
    }

    let user = UserRepo::update(
        &state.db,
        &claims.org,
        &id,
        UpdateUser {
            role: body.role,
            is_active: body.is_active,
        },
    )
    .await?;

    info!(
        user_id = %id,
        org_id = %claims.org,
        "👤 user updated"
    );

    Ok(Json(serde_json::json!({ "data": user })))
}

/// DELETE /api/v1/users/:id
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("users:delete") {
        return Err(ApiError::Forbidden);
    }
    // Prevent self-deactivation.
    if id == claims.sub {
        return Err(ApiError::BadRequest(
            "cannot deactivate your own account".into(),
        ));
    }
    UserRepo::deactivate(&state.db, &claims.org, &id).await?;
    info!(
        user_id = %id,
        org_id = %claims.org,
        "👤 user deactivated"
    );
    Ok(StatusCode::NO_CONTENT)
}
