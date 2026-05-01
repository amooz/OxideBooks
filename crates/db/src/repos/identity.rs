use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sqlx::PgPool;
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use oxidebooks_core::models::{
    CreatedScimToken, IdentityProvider, ProviderSummary, ProviderType, ScimToken,
};

use crate::error::{map_sqlx_err, DbError};

// ── IdentityProvider ──────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct IdpRow {
    id: Uuid,
    org_id: Uuid,
    name: String,
    provider_type: String,
    is_enabled: bool,
    email_domains: Vec<String>,
    oidc_client_id: Option<String>,
    oidc_client_secret: Option<String>,
    oidc_issuer_url: Option<String>,
    oidc_scopes: String,
    saml_idp_metadata_url: Option<String>,
    saml_idp_entity_id: Option<String>,
    saml_idp_sso_url: Option<String>,
    saml_idp_certificate: Option<String>,
    saml_sp_entity_id: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<IdpRow> for IdentityProvider {
    type Error = DbError;
    fn try_from(r: IdpRow) -> Result<Self, Self::Error> {
        let provider_type = ProviderType::from_str(&r.provider_type).map_err(DbError::Internal)?;
        Ok(IdentityProvider {
            id: r.id.to_string(),
            org_id: r.org_id.to_string(),
            name: r.name,
            provider_type,
            is_enabled: r.is_enabled,
            email_domains: r.email_domains,
            oidc_client_id: r.oidc_client_id,
            oidc_client_secret: r.oidc_client_secret,
            oidc_issuer_url: r.oidc_issuer_url,
            oidc_scopes: r.oidc_scopes,
            saml_idp_metadata_url: r.saml_idp_metadata_url,
            saml_idp_entity_id: r.saml_idp_entity_id,
            saml_idp_sso_url: r.saml_idp_sso_url,
            saml_idp_certificate: r.saml_idp_certificate,
            saml_sp_entity_id: r.saml_sp_entity_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

const IDP_COLUMNS: &str = "id, org_id, name, provider_type, is_enabled, email_domains, \
    oidc_client_id, oidc_client_secret, oidc_issuer_url, oidc_scopes, \
    saml_idp_metadata_url, saml_idp_entity_id, saml_idp_sso_url, saml_idp_certificate, \
    saml_sp_entity_id, created_at, updated_at";

pub struct IdentityProviderRepo;

impl IdentityProviderRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<IdentityProvider>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<IdpRow> = sqlx::query_as(&format!(
            "SELECT {IDP_COLUMNS} FROM identity_providers \
             WHERE org_id = $1 ORDER BY name"
        ))
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter().map(TryFrom::try_from).collect()
    }

    pub async fn get_by_id(
        pool: &PgPool,
        org_id: &str,
        id: &str,
    ) -> Result<IdentityProvider, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let row: IdpRow = sqlx::query_as(&format!(
            "SELECT {IDP_COLUMNS} FROM identity_providers \
             WHERE id = $1 AND org_id = $2"
        ))
        .bind(id_uuid)
        .bind(org_uuid)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        row.try_into()
    }

    pub async fn create_oidc(
        pool: &PgPool,
        org_id: &str,
        name: &str,
        client_id: &str,
        client_secret: &str,
        issuer_url: &str,
        scopes: &str,
        email_domains: &[String],
    ) -> Result<IdentityProvider, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let row: IdpRow = sqlx::query_as(&format!(
            "INSERT INTO identity_providers \
             (id, org_id, name, provider_type, oidc_client_id, oidc_client_secret, \
              oidc_issuer_url, oidc_scopes, email_domains) \
             VALUES ($1, $2, $3, 'oidc', $4, $5, $6, $7, $8) \
             RETURNING {IDP_COLUMNS}"
        ))
        .bind(id)
        .bind(org_uuid)
        .bind(name)
        .bind(client_id)
        .bind(client_secret)
        .bind(issuer_url)
        .bind(scopes)
        .bind(email_domains)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.try_into()
    }

    pub async fn create_saml(
        pool: &PgPool,
        org_id: &str,
        name: &str,
        idp_metadata_url: Option<&str>,
        idp_entity_id: Option<&str>,
        idp_sso_url: Option<&str>,
        idp_certificate: Option<&str>,
        sp_entity_id: Option<&str>,
        email_domains: &[String],
    ) -> Result<IdentityProvider, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let row: IdpRow = sqlx::query_as(&format!(
            "INSERT INTO identity_providers \
             (id, org_id, name, provider_type, saml_idp_metadata_url, saml_idp_entity_id, \
              saml_idp_sso_url, saml_idp_certificate, saml_sp_entity_id, email_domains) \
             VALUES ($1, $2, $3, 'saml', $4, $5, $6, $7, $8, $9) \
             RETURNING {IDP_COLUMNS}"
        ))
        .bind(id)
        .bind(org_uuid)
        .bind(name)
        .bind(idp_metadata_url)
        .bind(idp_entity_id)
        .bind(idp_sso_url)
        .bind(idp_certificate)
        .bind(sp_entity_id)
        .bind(email_domains)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        row.try_into()
    }

    pub async fn delete(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let result = sqlx::query("DELETE FROM identity_providers WHERE id = $1 AND org_id = $2")
            .bind(id_uuid)
            .bind(org_uuid)
            .execute(pool)
            .await
            .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Public-facing summary list (safe for the login page — no secrets).
    pub async fn list_summaries(
        pool: &PgPool,
        org_id: &str,
    ) -> Result<Vec<ProviderSummary>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            name: String,
            provider_type: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, name, provider_type FROM identity_providers \
             WHERE org_id = $1 AND is_enabled = TRUE ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        rows.into_iter()
            .map(|r| {
                Ok(ProviderSummary {
                    id: r.id.to_string(),
                    name: r.name,
                    provider_type: ProviderType::from_str(&r.provider_type)
                        .map_err(DbError::Internal)?,
                })
            })
            .collect()
    }

    /// Store OIDC state for the callback (anti-CSRF / PKCE).
    pub async fn store_oidc_state(
        pool: &PgPool,
        state: &str,
        provider_id: &str,
        org_id: &str,
        code_verifier: Option<&str>,
        post_login_uri: &str,
    ) -> Result<(), DbError> {
        let provider_uuid = parse_uuid(provider_id)?;
        let org_uuid = parse_uuid(org_id)?;

        sqlx::query(
            "INSERT INTO oidc_states (state, provider_id, org_id, code_verifier, post_login_uri) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(state)
        .bind(provider_uuid)
        .bind(org_uuid)
        .bind(code_verifier)
        .bind(post_login_uri)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    /// Consume the OIDC state (one-time use, deletes on retrieval).
    pub async fn consume_oidc_state(
        pool: &PgPool,
        state: &str,
    ) -> Result<(String, String, Option<String>, String), DbError> {
        #[derive(sqlx::FromRow)]
        struct StateRow {
            provider_id: Uuid,
            org_id: Uuid,
            code_verifier: Option<String>,
            post_login_uri: String,
            created_at: OffsetDateTime,
        }

        let row: StateRow = sqlx::query_as(
            "DELETE FROM oidc_states WHERE state = $1 RETURNING \
             provider_id, org_id, code_verifier, post_login_uri, created_at",
        )
        .bind(state)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .ok_or(DbError::NotFound)?;

        // Reject states older than 15 minutes.
        let age = OffsetDateTime::now_utc() - row.created_at;
        if age.whole_minutes() > 15 {
            return Err(DbError::Conflict("OIDC state expired".into()));
        }

        Ok((
            row.provider_id.to_string(),
            row.org_id.to_string(),
            row.code_verifier,
            row.post_login_uri,
        ))
    }
}

// ── ScimToken ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ScimTokenRow {
    id: Uuid,
    org_id: Uuid,
    name: String,
    is_active: bool,
    last_used_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

impl From<ScimTokenRow> for ScimToken {
    fn from(r: ScimTokenRow) -> Self {
        ScimToken {
            id: r.id.to_string(),
            org_id: r.org_id.to_string(),
            name: r.name,
            is_active: r.is_active,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
        }
    }
}

pub struct ScimTokenRepo;

impl ScimTokenRepo {
    pub async fn list(pool: &PgPool, org_id: &str) -> Result<Vec<ScimToken>, DbError> {
        let org_uuid = parse_uuid(org_id)?;

        let rows: Vec<ScimTokenRow> = sqlx::query_as(
            "SELECT id, org_id, name, is_active, last_used_at, created_at \
             FROM scim_tokens WHERE org_id = $1 ORDER BY name",
        )
        .bind(org_uuid)
        .fetch_all(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Create a token, returning both the metadata and the raw bearer token (shown once).
    pub async fn create(
        pool: &PgPool,
        org_id: &str,
        name: &str,
    ) -> Result<CreatedScimToken, DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id = Uuid::new_v4();

        let raw_token = generate_raw_token();
        let token_hash = hash_token(&raw_token).map_err(|e| DbError::Internal(e))?;

        let row: ScimTokenRow = sqlx::query_as(
            "INSERT INTO scim_tokens (id, org_id, name, token_hash) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, org_id, name, is_active, last_used_at, created_at",
        )
        .bind(id)
        .bind(org_uuid)
        .bind(name)
        .bind(&token_hash)
        .fetch_one(pool)
        .await
        .map_err(map_sqlx_err)?;

        let token: ScimToken = row.into();
        Ok(CreatedScimToken { raw_token, token })
    }

    /// Revoke (deactivate) a token.
    pub async fn revoke(pool: &PgPool, org_id: &str, id: &str) -> Result<(), DbError> {
        let org_uuid = parse_uuid(org_id)?;
        let id_uuid = parse_uuid(id)?;

        let result = sqlx::query(
            "UPDATE scim_tokens SET is_active = FALSE \
             WHERE id = $1 AND org_id = $2",
        )
        .bind(id_uuid)
        .bind(org_uuid)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    /// Verify a raw bearer token against all active tokens for the org.
    /// Returns the org_id if the token is valid, records last_used_at.
    pub async fn verify(pool: &PgPool, raw_token: &str) -> Result<String, DbError> {
        #[derive(sqlx::FromRow)]
        struct TokenHashRow {
            id: Uuid,
            org_id: Uuid,
            token_hash: String,
        }

        let rows: Vec<TokenHashRow> =
            sqlx::query_as("SELECT id, org_id, token_hash FROM scim_tokens WHERE is_active = TRUE")
                .fetch_all(pool)
                .await
                .map_err(map_sqlx_err)?;

        for row in &rows {
            if verify_token(raw_token, &row.token_hash) {
                // Record last_used_at (best-effort, don't fail the request).
                let _ = sqlx::query("UPDATE scim_tokens SET last_used_at = NOW() WHERE id = $1")
                    .bind(row.id)
                    .execute(pool)
                    .await;

                return Ok(row.org_id.to_string());
            }
        }

        Err(DbError::NotFound)
    }
}

// ── Token helpers ─────────────────────────────────────────────────────────────

fn generate_raw_token() -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("scim_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn hash_token(raw: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(raw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

fn verify_token(raw: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(raw.as_bytes(), &parsed)
        .is_ok()
}

fn parse_uuid(s: &str) -> Result<Uuid, DbError> {
    Uuid::parse_str(s).map_err(|_| DbError::Conflict(format!("invalid UUID: {s}")))
}
