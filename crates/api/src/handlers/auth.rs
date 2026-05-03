use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use oxidebooks_core::models::CreateOrganization;
use oxidebooks_db::repos::{
    organizations::OrganizationRepo,
    permissions::PermissionRepo,
    sessions::SessionRepo,
    users::{CreateUser, UserRepo},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub org_name: String,
    pub currency: Option<String>,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub org_id: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: String,
    pub org_id: String,
    pub role: String,
}

/// POST /api/v1/auth/register
/// Creates a new organization and owner account.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
    if !state.config.app.registration_open {
        return Err(ApiError::Forbidden);
    }
    if body.password.len() < 12 {
        return Err(ApiError::BadRequest(
            "password must be at least 12 characters".into(),
        ));
    }

    let password_hash = hash_password(&body.password)?;

    let org = OrganizationRepo::create(
        &state.db,
        CreateOrganization {
            name: body.org_name,
            currency: body
                .currency
                .unwrap_or_else(|| state.config.app.default_currency.clone()),
            fiscal_year_start: None,
        },
    )
    .await?;

    let user = UserRepo::create(
        &state.db,
        CreateUser {
            organization_id: org.id.clone(),
            email: body.email,
            password_hash,
            name: body.name,
            role: "owner".to_string(),
        },
    )
    .await?;

    let token = mint_token(&state.db, &user.id, &org.id, &user.role, &state).await?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        org_id: org.id,
        role: user.role,
    }))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let record = UserRepo::get_by_email(&state.db, &body.org_id, &body.email)
        .await
        .map_err(|_| ApiError::Unauthorized)?;

    verify_password(&body.password, &record.password_hash)?;

    let token = mint_token(
        &state.db,
        &record.user.id,
        &record.user.organization_id,
        &record.user.role,
        &state,
    )
    .await?;

    Ok(Json(AuthResponse {
        token,
        user_id: record.user.id,
        org_id: record.user.organization_id,
        role: record.user.role,
    }))
}

fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password hashing failed: {e}")))
}

fn verify_password(password: &str, hash: &str) -> ApiResult<()> {
    let parsed = PasswordHash::new(hash).map_err(|_| ApiError::Unauthorized)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| ApiError::Unauthorized)
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/v1/auth/password
/// Allows an authenticated user to change their own password.
pub async fn change_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    if body.new_password.len() < 12 {
        return Err(ApiError::BadRequest(
            "new password must be at least 12 characters".into(),
        ));
    }

    let record = UserRepo::get_by_id_with_hash(&state.db, &claims.sub)
        .await
        .map_err(|_| ApiError::Unauthorized)?;

    if record.password_hash.is_empty() {
        return Err(ApiError::BadRequest(
            "account uses SSO authentication".into(),
        ));
    }

    verify_password(&body.current_password, &record.password_hash)?;

    let new_hash = hash_password(&body.new_password)?;
    UserRepo::update_password(&state.db, &claims.sub, &new_hash).await?;

    info!(user_id = %claims.sub, "🔐 password changed");

    Ok(StatusCode::NO_CONTENT)
}

async fn mint_token(
    pool: &PgPool,
    user_id: &str,
    org_id: &str,
    role: &str,
    state: &AppState,
) -> ApiResult<String> {
    let permissions = PermissionRepo::list_for_user(pool, user_id)
        .await
        .map_err(ApiError::Db)?;

    let claims = Claims::new(
        user_id,
        org_id,
        role,
        permissions,
        state.config.auth.token_expiry_hours,
    );

    // Store session for revocation support (fire-and-forget — a failure here
    // does not prevent login)
    let pool2 = pool.clone();
    let jti = claims.jti.clone();
    let uid = user_id.to_string();
    let oid = org_id.to_string();
    let expires_at = claims.expires_at();
    tokio::spawn(async move {
        let _ = SessionRepo::create(&pool2, &uid, &oid, &jti, expires_at, None, None).await;
    });

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.auth.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("token signing failed: {e}")))
}
