/// SCIM 2.0 user provisioning endpoint.
///
/// Base path: `/scim/v2/`
/// Authentication: Bearer token validated against the `scim_tokens` table.
///
/// Supports the SCIM 2.0 core schema for Users as defined in RFC 7644.
/// Clients (e.g., Okta, Azure AD, JumpCloud) can use these endpoints to
/// automatically provision and deprovision users.
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use oxidebooks_db::repos::users::CreateUser;
use oxidebooks_db::repos::{ScimTokenRepo, UserRepo};

use crate::{error::ApiError, state::AppState};

fn hash_password_scim(password: &str) -> Option<String> {
    if password.len() < 12 {
        return None;
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .ok()
        .map(|h| h.to_string())
}

// ── SCIM schema constants ─────────────────────────────────────────────────────

const SCIM_USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
const SCIM_LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
const SCIM_ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";

// ── SCIM wire types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScimUserInput {
    pub user_name: String,
    pub display_name: Option<String>,
    pub active: Option<bool>,
    pub name: Option<ScimName>,
    pub emails: Option<Vec<ScimEmail>>,
    #[serde(skip_serializing)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScimName {
    pub formatted: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScimEmail {
    pub value: String,
    pub primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ScimPatchOp {
    pub operations: Vec<ScimOperation>,
}

#[derive(Debug, Deserialize)]
pub struct ScimOperation {
    pub op: String,
    pub path: Option<String>,
    pub value: Option<Value>,
}

// ── SCIM auth helper ──────────────────────────────────────────────────────────

async fn extract_scim_org(state: &AppState, headers: &HeaderMap) -> Result<String, ApiError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    ScimTokenRepo::verify(&state.db, token)
        .await
        .map_err(|_| ApiError::Unauthorized)
}

fn scim_error(status: StatusCode, detail: &str) -> impl IntoResponse {
    (
        status,
        Json(json!({
            "schemas": [SCIM_ERROR_SCHEMA],
            "status": status.as_u16(),
            "detail": detail
        })),
    )
}

// ── ServiceProviderConfig ─────────────────────────────────────────────────────

/// GET /scim/v2/ServiceProviderConfig
pub async fn service_provider_config() -> impl IntoResponse {
    Json(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
        "documentationUri": "https://oxidebooks.example.com/docs/scim",
        "patch": { "supported": true },
        "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
        "filter": { "supported": true, "maxResults": 200 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
        "authenticationSchemes": [{
            "type": "oauthbearertoken",
            "name": "OAuth Bearer Token",
            "description": "Authentication scheme using the OAuth Bearer Token standard"
        }]
    }))
}

// ── Users ─────────────────────────────────────────────────────────────────────

/// GET /scim/v2/Users
pub async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let org_id = match extract_scim_org(&state, &headers).await {
        Ok(id) => id,
        Err(_) => {
            return scim_error(StatusCode::UNAUTHORIZED, "invalid SCIM token").into_response()
        }
    };

    let rows: Vec<(uuid::Uuid, String, String, String, bool)> = match sqlx::query_as(
        "SELECT u.id, u.email, u.name, r.name, u.is_active \
         FROM users u \
         JOIN roles r ON r.id = u.role_id \
         WHERE u.organization_id = $1 \
         ORDER BY u.email",
    )
    .bind(uuid::Uuid::parse_str(&org_id).unwrap_or_default())
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("SCIM list_users: {e}");
            return scim_error(StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response();
        }
    };

    let resources: Vec<Value> = rows
        .into_iter()
        .map(|(id, email, name, role, active)| {
            scim_user_resource(&id.to_string(), &email, &name, &role, active)
        })
        .collect();

    let count = resources.len();
    Json(json!({
        "schemas": [SCIM_LIST_SCHEMA],
        "totalResults": count,
        "startIndex": 1,
        "itemsPerPage": count,
        "Resources": resources,
    }))
    .into_response()
}

/// GET /scim/v2/Users/:id
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _org_id = match extract_scim_org(&state, &headers).await {
        Ok(id) => id,
        Err(_) => {
            return scim_error(StatusCode::UNAUTHORIZED, "invalid SCIM token").into_response()
        }
    };

    match UserRepo::get_by_id(&state.db, &user_id).await {
        Ok(u) => Json(scim_user_resource(
            &u.id,
            &u.email,
            &u.name,
            &u.role,
            u.is_active,
        ))
        .into_response(),
        Err(_) => scim_error(StatusCode::NOT_FOUND, "user not found").into_response(),
    }
}

/// POST /scim/v2/Users
pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let org_id = match extract_scim_org(&state, &headers).await {
        Ok(id) => id,
        Err(_) => {
            return scim_error(StatusCode::UNAUTHORIZED, "invalid SCIM token").into_response()
        }
    };

    let input: ScimUserInput = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return scim_error(StatusCode::BAD_REQUEST, &format!("invalid JSON: {e}"))
                .into_response()
        }
    };

    let email = input
        .emails
        .and_then(|es| es.into_iter().find(|e| e.primary.unwrap_or(false)))
        .map(|e| e.value)
        .unwrap_or_else(|| input.user_name.clone());

    let name = input
        .display_name
        .or_else(|| {
            input.name.as_ref().and_then(|n| {
                n.formatted
                    .clone()
                    .or_else(|| match (&n.given_name, &n.family_name) {
                        (Some(g), Some(f)) => Some(format!("{g} {f}")),
                        (Some(g), None) => Some(g.clone()),
                        _ => None,
                    })
            })
        })
        .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

    // Hash password if provided; reject if it fails the complexity check.
    let password_hash = if let Some(ref pw) = input.password {
        match hash_password_scim(pw) {
            Some(h) => h,
            None => {
                return scim_error(
                    StatusCode::BAD_REQUEST,
                    "password must be at least 12 characters",
                )
                .into_response()
            }
        }
    } else {
        "".to_string()
    };

    let user = match UserRepo::create(
        &state.db,
        CreateUser {
            organization_id: org_id.clone(),
            email: email.clone(),
            password_hash,
            name: name.clone(),
            role: "viewer".to_string(),
        },
    )
    .await
    {
        Ok(u) => u,
        Err(e) if e.is_conflict() => {
            return scim_error(StatusCode::CONFLICT, "user already exists").into_response()
        }
        Err(e) => {
            tracing::error!("SCIM create_user: {e}");
            return scim_error(StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response();
        }
    };

    (
        StatusCode::CREATED,
        Json(scim_user_resource(
            &user.id,
            &user.email,
            &user.name,
            &user.role,
            user.is_active,
        )),
    )
        .into_response()
}

/// PATCH /scim/v2/Users/:id
/// Handles SCIM patch operations (typically active = true/false for de-provisioning).
pub async fn patch_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let _org_id = match extract_scim_org(&state, &headers).await {
        Ok(id) => id,
        Err(_) => {
            return scim_error(StatusCode::UNAUTHORIZED, "invalid SCIM token").into_response()
        }
    };

    let patch: ScimPatchOp = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return scim_error(StatusCode::BAD_REQUEST, &format!("invalid JSON: {e}"))
                .into_response()
        }
    };

    let user_uuid = match uuid::Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => return scim_error(StatusCode::NOT_FOUND, "user not found").into_response(),
    };

    for op in &patch.operations {
        match op.op.to_lowercase().as_str() {
            "replace" => {
                // active flag
                let active = match op.path.as_deref() {
                    Some("active") => op.value.as_ref().and_then(|v| v.as_bool()),
                    _ => op
                        .value
                        .as_ref()
                        .and_then(|v| v.get("active"))
                        .and_then(|v| v.as_bool()),
                };
                if let Some(is_active) = active {
                    let _ = sqlx::query("UPDATE users SET is_active = $1 WHERE id = $2")
                        .bind(is_active)
                        .bind(user_uuid)
                        .execute(&state.db)
                        .await;
                }

                // password
                if op.path.as_deref() == Some("password") {
                    if let Some(pw) = op.value.as_ref().and_then(|v| v.as_str()) {
                        if let Some(hash) = hash_password_scim(pw) {
                            let _ =
                                sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
                                    .bind(&hash)
                                    .bind(user_uuid)
                                    .execute(&state.db)
                                    .await;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    match UserRepo::get_by_id(&state.db, &user_id).await {
        Ok(u) => Json(scim_user_resource(
            &u.id,
            &u.email,
            &u.name,
            &u.role,
            u.is_active,
        ))
        .into_response(),
        Err(_) => scim_error(StatusCode::NOT_FOUND, "user not found").into_response(),
    }
}

/// DELETE /scim/v2/Users/:id
/// Deactivates (does not hard-delete) the user.
pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _org_id = match extract_scim_org(&state, &headers).await {
        Ok(id) => id,
        Err(_) => {
            return scim_error(StatusCode::UNAUTHORIZED, "invalid SCIM token").into_response()
        }
    };

    let user_uuid = match uuid::Uuid::parse_str(&user_id) {
        Ok(u) => u,
        Err(_) => return scim_error(StatusCode::NOT_FOUND, "user not found").into_response(),
    };

    match sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(user_uuid)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            scim_error(StatusCode::NOT_FOUND, "user not found").into_response()
        }
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("SCIM delete_user: {e}");
            scim_error(StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
        }
    }
}

// ── Serialization helpers ─────────────────────────────────────────────────────

fn scim_user_resource(id: &str, email: &str, name: &str, role: &str, active: bool) -> Value {
    json!({
        "schemas": [SCIM_USER_SCHEMA],
        "id": id,
        "userName": email,
        "displayName": name,
        "active": active,
        "emails": [{ "value": email, "primary": true }],
        "roles": [{ "value": role }],
        "meta": {
            "resourceType": "User",
            "location": format!("/scim/v2/Users/{id}")
        }
    })
}
