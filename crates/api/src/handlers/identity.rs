/// Identity provider management (OIDC and SAML configuration).
///
/// These endpoints let admins configure external SSO providers per org.
/// The actual auth flows live in `auth_sso.rs`.
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use oxidebooks_core::models::{CreateOidcProvider, CreateSamlProvider, CreateScimToken};
use oxidebooks_db::repos::{IdentityProviderRepo, ScimTokenRepo};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

// ── Identity provider management ─────────────────────────────────────────────

/// GET /api/v1/identity-providers
pub async fn list_providers(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("users:read") {
        return Err(ApiError::Forbidden);
    }
    let providers = IdentityProviderRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": providers })))
}

/// POST /api/v1/identity-providers/oidc
pub async fn create_oidc_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateOidcProvider>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }
    let scopes = body.scopes.as_deref().unwrap_or("openid email profile");
    let domains = body.email_domains.as_deref().unwrap_or(&[]);

    let provider = IdentityProviderRepo::create_oidc(
        &state.db,
        &claims.org,
        &body.name,
        &body.client_id,
        &body.client_secret,
        &body.issuer_url,
        scopes,
        domains,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": provider })),
    ))
}

/// POST /api/v1/identity-providers/saml
pub async fn create_saml_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateSamlProvider>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }
    let domains = body.email_domains.as_deref().unwrap_or(&[]);

    let provider = IdentityProviderRepo::create_saml(
        &state.db,
        &claims.org,
        &body.name,
        body.idp_metadata_url.as_deref(),
        body.idp_entity_id.as_deref(),
        body.idp_sso_url.as_deref(),
        body.idp_certificate.as_deref(),
        body.sp_entity_id.as_deref(),
        domains,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": provider })),
    ))
}

/// DELETE /api/v1/identity-providers/:id
pub async fn delete_provider(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }
    IdentityProviderRepo::delete(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── SCIM token management ─────────────────────────────────────────────────────

/// GET /api/v1/scim/tokens
pub async fn list_scim_tokens(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    if !claims.has("users:read") {
        return Err(ApiError::Forbidden);
    }
    let tokens = ScimTokenRepo::list(&state.db, &claims.org).await?;
    Ok(Json(serde_json::json!({ "data": tokens })))
}

/// POST /api/v1/scim/tokens
pub async fn create_scim_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateScimToken>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }
    if body.name.trim().is_empty() {
        return Err(ApiError::BadRequest("token name must not be empty".into()));
    }
    let created = ScimTokenRepo::create(&state.db, &claims.org, body.name.trim()).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": created })),
    ))
}

/// DELETE /api/v1/scim/tokens/:id  (revokes the token)
pub async fn revoke_scim_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    if !claims.has("users:write") {
        return Err(ApiError::Forbidden);
    }
    ScimTokenRepo::revoke(&state.db, &claims.org, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
