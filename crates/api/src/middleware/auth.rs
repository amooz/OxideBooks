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
    /// Subject — user ID.
    pub sub: String,
    /// Organization ID the user belongs to.
    pub org: String,
    /// Role name, for display only (e.g. "owner", "accountant").
    pub role: String,
    /// Resolved permission names for this user's role.
    pub permissions: Vec<String>,
    /// Expiry as Unix timestamp.
    pub exp: usize,
}

impl Claims {
    pub fn new(
        user_id: &str,
        org_id: &str,
        role: &str,
        permissions: Vec<String>,
        expiry_hours: i64,
    ) -> Self {
        let exp = (OffsetDateTime::now_utc() + time::Duration::hours(expiry_hours)).unix_timestamp()
            as usize;
        Self {
            sub: user_id.to_string(),
            org: org_id.to_string(),
            role: role.to_string(),
            permissions,
            exp,
        }
    }

    /// Returns true if the caller holds the named permission.
    pub fn has(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Returns true if the caller is admin or owner.
    pub fn is_admin(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "owner")
    }

    /// Returns true if the caller is accountant, admin, or owner.
    pub fn is_at_least_accountant(&self) -> bool {
        matches!(self.role.as_str(), "accountant" | "admin" | "owner")
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
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims_with(permissions: Vec<&str>) -> Claims {
        Claims {
            sub: "user-1".into(),
            org: "org-1".into(),
            role: "custom".into(),
            permissions: permissions.into_iter().map(String::from).collect(),
            exp: 9_999_999_999,
        }
    }

    #[test]
    fn has_returns_true_for_held_permission() {
        let c = claims_with(vec!["accounts:read", "accounts:write"]);
        assert!(c.has("accounts:read"));
        assert!(c.has("accounts:write"));
    }

    #[test]
    fn has_returns_false_for_missing_permission() {
        let c = claims_with(vec!["accounts:read"]);
        assert!(!c.has("accounts:delete"));
        assert!(!c.has("roles:write"));
    }

    #[test]
    fn has_returns_false_for_empty_permissions() {
        let c = claims_with(vec![]);
        assert!(!c.has("accounts:read"));
    }

    #[test]
    fn has_is_exact_match_not_prefix() {
        let c = claims_with(vec!["accounts:read"]);
        assert!(!c.has("accounts"));
        assert!(!c.has("accounts:rea"));
        assert!(!c.has("accounts:readwrite"));
    }

    #[test]
    fn owner_all_permissions_example() {
        let all = vec![
            "accounts:read",
            "accounts:write",
            "accounts:delete",
            "transactions:read",
            "transactions:write",
            "contacts:read",
            "contacts:write",
            "invoices:read",
            "invoices:write",
            "reports:read",
            "users:read",
            "users:write",
            "users:delete",
            "roles:read",
            "roles:write",
        ];
        let c = claims_with(all.clone());
        for p in all {
            assert!(c.has(p), "owner should have {p}");
        }
    }

    #[test]
    fn viewer_cannot_write() {
        let c = claims_with(vec![
            "accounts:read",
            "transactions:read",
            "contacts:read",
            "invoices:read",
            "reports:read",
        ]);
        assert!(c.has("accounts:read"));
        assert!(!c.has("accounts:write"));
        assert!(!c.has("accounts:delete"));
        assert!(!c.has("roles:write"));
    }
}
