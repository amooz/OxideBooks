use axum::{
    extract::{Extension, State},
    http::StatusCode,
    Json,
};
use oxidebooks_db::repos::UserRepo;
use serde::Deserialize;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::{
    error::{ApiError, ApiResult},
    middleware::Claims,
    state::AppState,
};

pub async fn setup_totp(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Json<serde_json::Value>> {
    let raw_secret = Secret::generate_secret();
    let base32_secret = raw_secret.to_encoded().to_string();
    let secret_bytes = raw_secret
        .to_bytes()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("OxideBooks".to_string()),
        claims.sub.clone(),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    UserRepo::set_totp_secret(&state.db, &claims.sub, &base32_secret).await?;

    Ok(Json(serde_json::json!({
        "secret": base32_secret,
        "uri": totp.get_url(),
    })))
}

#[derive(Deserialize)]
pub struct VerifyTotpBody {
    pub code: String,
}

pub async fn verify_totp(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<VerifyTotpBody>,
) -> ApiResult<StatusCode> {
    let stored = UserRepo::get_totp_secret(&state.db, &claims.sub)
        .await?
        .ok_or(ApiError::BadRequest("TOTP not set up".into()))?;

    let secret_bytes = Secret::Encoded(stored)
        .to_bytes()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("OxideBooks".to_string()),
        claims.sub.clone(),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let valid = totp
        .check_current(&body.code)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    if !valid {
        return Err(ApiError::BadRequest("invalid TOTP code".into()));
    }

    UserRepo::enable_totp(&state.db, &claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn disable_totp(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<StatusCode> {
    UserRepo::disable_totp(&state.db, &claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}
