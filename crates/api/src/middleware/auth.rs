use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{error::ApiError, state::AppState};

/// JWT claims embedded in every authenticated request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user ID
    pub sub: String,
    /// Organization ID the user belongs to
    pub org: String,
    /// User role (owner / admin / accountant / viewer)
    pub role: String,
    /// Expiry as Unix timestamp
    pub exp: usize,
}

impl Claims {
    pub fn new(user_id: &str, org_id: &str, role: &str, expiry_hours: i64) -> Self {
        let exp = (OffsetDateTime::now_utc() + time::Duration::hours(expiry_hours))
            .unix_timestamp() as usize;
        Self {
            sub: user_id.to_string(),
            org: org_id.to_string(),
            role: role.to_string(),
            exp,
        }
    }

    pub fn is_at_least_accountant(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "admin" | "accountant")
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "admin")
    }
}

/// Axum middleware: validates Bearer token and injects [`Claims`] as a request extension.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;

    let secret = state.config.auth.jwt_secret.as_bytes();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &Validation::default())
        .map_err(|_| ApiError::Unauthorized)?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}
