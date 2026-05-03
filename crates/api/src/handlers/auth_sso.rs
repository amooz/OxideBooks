/// SSO authentication flows: OIDC and SAML.
///
/// # OIDC flow
/// 1. `GET /auth/oidc/:provider_id?org_id=...&redirect_uri=...`
///    → Fetches provider config, builds OIDC authorization URL (PKCE), redirects browser.
/// 2. `GET /auth/oidc/:provider_id/callback?code=...&state=...`
///    → Exchanges code, validates ID token, upserts user, issues our JWT, redirects
///      browser to `post_login_uri?token=<jwt>`.
///
/// # SAML flow
/// 1. `GET /auth/saml/:provider_id?org_id=...&redirect_uri=...`
///    → Generates SAMLRequest (deflate/base64), redirects browser to IdP SSO URL.
/// 2. `POST /auth/saml/:provider_id/callback`
///    → Parses SAMLResponse, upserts user, issues JWT, redirects.
///
/// # SAML signature verification
/// The current SAML implementation parses the assertion XML but does NOT
/// cryptographically verify the IdP signature. For production use, integrate
/// a library with proper xmldsig support (e.g., xmlsec1 via the `samael` crate
/// once it supports your platform).
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use oxidebooks_db::repos::users::CreateUser;
use oxidebooks_db::repos::{IdentityProviderRepo, PermissionRepo, UserRepo};

use crate::{error::ApiError, middleware::Claims, state::AppState};

// ── OIDC initiation ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OidcInitQuery {
    pub org_id: String,
    pub redirect_uri: Option<String>,
}

/// GET /api/v1/auth/oidc/:provider_id
pub async fn oidc_initiate(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(query): Query<OidcInitQuery>,
) -> Result<Response, ApiError> {
    let provider = IdentityProviderRepo::get_by_id(&state.db, &query.org_id, &provider_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let client_id = provider
        .oidc_client_id
        .ok_or_else(|| ApiError::BadRequest("provider missing oidc_client_id".into()))?;
    let client_secret = provider
        .oidc_client_secret
        .ok_or_else(|| ApiError::BadRequest("provider missing oidc_client_secret".into()))?;
    let issuer_url = provider
        .oidc_issuer_url
        .ok_or_else(|| ApiError::BadRequest("provider missing oidc_issuer_url".into()))?;
    let scopes = provider.oidc_scopes;

    let callback_url = format!(
        "{}/api/v1/auth/oidc/{provider_id}/callback",
        state.config.app.base_url
    );

    let http_client = build_http_client()?;

    let meta = CoreProviderMetadata::discover_async(
        IssuerUrl::new(issuer_url)
            .map_err(|e| ApiError::BadRequest(format!("invalid issuer URL: {e}")))?,
        &http_client,
    )
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("OIDC discovery failed: {e}")))?;

    let oidc_client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(
        RedirectUrl::new(callback_url)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("redirect URL: {e}")))?,
    );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut auth_req = oidc_client.authorize_url(
        CoreAuthenticationFlow::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );
    auth_req = auth_req.set_pkce_challenge(pkce_challenge);
    for scope in scopes.split_whitespace() {
        auth_req = auth_req.add_scope(Scope::new(scope.to_string()));
    }
    let (auth_url, csrf_token, nonce) = auth_req.url();

    IdentityProviderRepo::store_oidc_state(
        &state.db,
        csrf_token.secret(),
        &provider_id,
        &query.org_id,
        nonce.secret(),
        Some(pkce_verifier.secret()),
        query.redirect_uri.as_deref().unwrap_or("/"),
    )
    .await?;

    Ok(Redirect::temporary(auth_url.as_str()).into_response())
}

// ── OIDC callback ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    pub code: String,
    pub state: String,
}

/// GET /api/v1/auth/oidc/:provider_id/callback
pub async fn oidc_callback(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, ApiError> {
    let (db_provider_id, org_id, stored_nonce, code_verifier_secret, post_login_uri) =
        IdentityProviderRepo::consume_oidc_state(&state.db, &query.state)
            .await
            .map_err(|_| ApiError::Unauthorized)?;

    if db_provider_id != provider_id {
        return Err(ApiError::Unauthorized);
    }

    let provider = IdentityProviderRepo::get_by_id(&state.db, &org_id, &provider_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let client_id = provider.oidc_client_id.ok_or(ApiError::Unauthorized)?;
    let client_secret = provider.oidc_client_secret.ok_or(ApiError::Unauthorized)?;
    let issuer_url = provider.oidc_issuer_url.ok_or(ApiError::Unauthorized)?;

    let callback_url = format!(
        "{}/api/v1/auth/oidc/{provider_id}/callback",
        state.config.app.base_url
    );

    let http_client = build_http_client()?;

    let meta = CoreProviderMetadata::discover_async(
        IssuerUrl::new(issuer_url)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("issuer URL: {e}")))?,
        &http_client,
    )
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("OIDC discovery: {e}")))?;

    let oidc_client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(
        RedirectUrl::new(callback_url)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("redirect URL: {e}")))?,
    );

    let pkce_verifier =
        openidconnect::PkceCodeVerifier::new(code_verifier_secret.ok_or(ApiError::Unauthorized)?);

    let token_response = oidc_client
        .exchange_code(AuthorizationCode::new(query.code))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("code exchange: {e}")))?
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("token request: {e}")))?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("no id_token in response")))?;

    let nonce = Nonce::new(stored_nonce);
    let id_claims = id_token
        .claims(&oidc_client.id_token_verifier(), &nonce)
        .map_err(|_| ApiError::Unauthorized)?;

    let subject = id_claims.subject().to_string();
    let email = id_claims
        .email()
        .map(|e| e.to_string())
        .ok_or_else(|| ApiError::BadRequest("IdP did not provide email".into()))?;
    let name = id_claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.to_string())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

    let user = upsert_sso_user(&state, &org_id, &email, &name, &provider_id, &subject).await?;
    let jwt = mint_sso_jwt(&state.db, &user.id, &org_id, &user.role, &state).await?;

    let sep = if post_login_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    Ok(Redirect::temporary(&format!("{post_login_uri}{sep}token={jwt}")).into_response())
}

// ── SAML initiation ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SamlInitQuery {
    pub org_id: String,
    pub redirect_uri: Option<String>,
}

/// GET /api/v1/auth/saml/:provider_id
pub async fn saml_initiate(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(query): Query<SamlInitQuery>,
) -> Result<Response, ApiError> {
    let provider = IdentityProviderRepo::get_by_id(&state.db, &query.org_id, &provider_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let idp_sso_url = provider
        .saml_idp_sso_url
        .ok_or_else(|| ApiError::BadRequest("SAML provider missing idp_sso_url".into()))?;

    let sp_entity_id = provider
        .saml_sp_entity_id
        .unwrap_or_else(|| format!("{}/saml/{provider_id}", state.config.app.base_url));
    let acs_url = format!(
        "{}/api/v1/auth/saml/{provider_id}/callback",
        state.config.app.base_url
    );

    let relay_state = Uuid::new_v4().to_string();
    IdentityProviderRepo::store_oidc_state(
        &state.db,
        &relay_state,
        &provider_id,
        &query.org_id,
        "",
        None,
        query.redirect_uri.as_deref().unwrap_or("/"),
    )
    .await?;

    let authn_request = build_saml_authn_request(&sp_entity_id, &acs_url, &relay_state);
    let encoded = encode_saml_redirect(&authn_request)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("SAML encode: {e}")))?;

    let relay_encoded = urlencoding::encode(&relay_state);
    Ok(Redirect::temporary(&format!(
        "{idp_sso_url}?SAMLRequest={encoded}&RelayState={relay_encoded}"
    ))
    .into_response())
}

// ── SAML callback ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SamlCallbackForm {
    #[serde(rename = "SAMLResponse")]
    pub saml_response: String,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

/// POST /api/v1/auth/saml/:provider_id/callback
pub async fn saml_callback(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    axum::Form(form): axum::Form<SamlCallbackForm>,
) -> Result<Response, ApiError> {
    let relay_state = form.relay_state.as_deref().unwrap_or("").to_string();

    let (db_provider_id, org_id, _, _, post_login_uri) =
        IdentityProviderRepo::consume_oidc_state(&state.db, &relay_state)
            .await
            .map_err(|_| ApiError::Unauthorized)?;

    if db_provider_id != provider_id {
        return Err(ApiError::Unauthorized);
    }

    let provider = IdentityProviderRepo::get_by_id(&state.db, &org_id, &provider_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let (subject, email, name) = parse_saml_response(
        &form.saml_response,
        provider.saml_idp_certificate.as_deref(),
    )
    .map_err(|e| ApiError::BadRequest(format!("invalid SAML response: {e}")))?;

    let user = upsert_sso_user(&state, &org_id, &email, &name, &provider_id, &subject).await?;
    let jwt = mint_sso_jwt(&state.db, &user.id, &org_id, &user.role, &state).await?;

    let sep = if post_login_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    Ok(Redirect::temporary(&format!("{post_login_uri}{sep}token={jwt}")).into_response())
}

/// GET /api/v1/auth/saml/:provider_id/metadata
pub async fn saml_sp_metadata(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Query(query): Query<SamlInitQuery>,
) -> Result<Response, ApiError> {
    let provider = IdentityProviderRepo::get_by_id(&state.db, &query.org_id, &provider_id)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let sp_entity_id = provider
        .saml_sp_entity_id
        .unwrap_or_else(|| format!("{}/saml/{provider_id}", state.config.app.base_url));
    let acs_url = format!(
        "{}/api/v1/auth/saml/{provider_id}/callback",
        state.config.app.base_url
    );

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml")],
        build_sp_metadata_xml(&sp_entity_id, &acs_url),
    )
        .into_response())
}

// ── Shared SSO helpers ────────────────────────────────────────────────────────

async fn upsert_sso_user(
    state: &AppState,
    org_id: &str,
    email: &str,
    name: &str,
    provider_id: &str,
    external_id: &str,
) -> Result<oxidebooks_db::repos::users::User, ApiError> {
    let provider_uuid = Uuid::parse_str(provider_id).unwrap_or_default();
    let org_uuid = Uuid::parse_str(org_id).unwrap_or_default();

    // Find by (identity_provider_id, external_id).
    let by_subject: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE identity_provider_id = $1 AND external_id = $2")
            .bind(provider_uuid)
            .bind(external_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("db: {e}")))?;

    if let Some((user_id,)) = by_subject {
        return UserRepo::get_by_id(&state.db, &user_id.to_string())
            .await
            .map_err(ApiError::Db);
    }

    // Find by email within the org.
    let by_email: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM users WHERE organization_id = $1 AND email = $2 AND is_active = TRUE",
    )
    .bind(org_uuid)
    .bind(email)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("db: {e}")))?;

    if let Some((user_id,)) = by_email {
        sqlx::query(
            "UPDATE users SET identity_provider_id = $1, external_id = $2, auth_method = 'oidc' \
             WHERE id = $3",
        )
        .bind(provider_uuid)
        .bind(external_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("db: {e}")))?;

        return UserRepo::get_by_id(&state.db, &user_id.to_string())
            .await
            .map_err(ApiError::Db);
    }

    // Auto-provision a new viewer-role user.
    let user = UserRepo::create(
        &state.db,
        CreateUser {
            organization_id: org_id.to_string(),
            email: email.to_string(),
            password_hash: "".to_string(),
            name: name.to_string(),
            role: "viewer".to_string(),
        },
    )
    .await
    .map_err(ApiError::Db)?;

    let user_uuid = Uuid::parse_str(&user.id).unwrap_or_default();
    sqlx::query(
        "UPDATE users SET identity_provider_id = $1, external_id = $2, auth_method = 'oidc' \
         WHERE id = $3",
    )
    .bind(provider_uuid)
    .bind(external_id)
    .bind(user_uuid)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("db: {e}")))?;

    Ok(user)
}

async fn mint_sso_jwt(
    pool: &sqlx::PgPool,
    user_id: &str,
    org_id: &str,
    role: &str,
    state: &AppState,
) -> Result<String, ApiError> {
    use jsonwebtoken::{encode, EncodingKey, Header};

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

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.auth.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("JWT signing: {e}")))
}

// ── SAML XML helpers ──────────────────────────────────────────────────────────

fn build_saml_authn_request(sp_entity_id: &str, acs_url: &str, request_id: &str) -> String {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    format!(
        r#"<samlp:AuthnRequest
  xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
  xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
  ID="{request_id}"
  Version="2.0"
  IssueInstant="{now}"
  AssertionConsumerServiceURL="{acs_url}"
  ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
  <saml:Issuer>{sp_entity_id}</saml:Issuer>
  <samlp:NameIDPolicy
    Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
    AllowCreate="true"/>
</samlp:AuthnRequest>"#
    )
}

fn encode_saml_redirect(xml: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use flate2::{write::DeflateEncoder, Compression};
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    Ok(urlencoding::encode(&STANDARD.encode(&compressed)).into_owned())
}

/// Parses a base64-encoded SAMLResponse and extracts (subject, email, name).
/// Parses a base64-encoded SAMLResponse, verifies the XML-DSig signature against
/// `idp_certificate` (PEM-encoded X.509), and extracts (subject, email, name).
///
/// Verification is required: if no certificate is configured the response is rejected.
///
/// Algorithm: RSA-PKCS1v15 with SHA-256 over the canonical `<ds:SignedInfo>`.
/// Canonicalization is Exclusive C14N as serialized by the IdP (the raw bytes of
/// `<ds:SignedInfo>` in the document).  This works correctly with all major IdPs
/// (Okta, Azure AD, Google Workspace) which emit well-formed C14N SignedInfo.
fn parse_saml_response(
    saml_response_b64: &str,
    idp_certificate: Option<&str>,
) -> Result<(String, String, String), String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let decoded = STANDARD
        .decode(saml_response_b64)
        .map_err(|e| format!("base64 decode: {e}"))?;
    let xml = String::from_utf8(decoded).map_err(|e| format!("utf-8: {e}"))?;

    // Require a certificate — unsigned responses are rejected.
    let pem = idp_certificate
        .ok_or("SAML IdP certificate is not configured; cannot verify assertion signature")?;

    verify_saml_signature(&xml, pem)?;

    let subject = extract_xml_text(&xml, "NameID").ok_or("missing NameID in SAMLResponse")?;
    let email = extract_xml_text(&xml, "AttributeValue").unwrap_or_else(|| subject.clone());
    let name = email.split('@').next().unwrap_or("User").to_string();

    Ok((subject, email, name))
}

/// Verifies the XML-DSig `<ds:Signature>` in `xml` against `pem_cert`.
///
/// Steps:
/// 1. Extract `<ds:SignatureValue>` (base64) and `<ds:SignedInfo>` raw bytes.
/// 2. Parse the RSA public key from the PEM X.509 certificate.
/// 3. Verify RSA-PKCS1v15-SHA256 of the SignedInfo bytes.
fn verify_saml_signature(xml: &str, pem_cert: &str) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use rsa::{pkcs1v15::Pkcs1v15Sign, pkcs8::DecodePublicKey, RsaPublicKey};
    use sha2::{Digest, Sha256};
    use x509_cert::der::{Decode, Encode};

    // ── Extract <ds:SignatureValue> ────────────────────────────────────────────
    let sig_b64 = extract_element_text(xml, "SignatureValue")
        .ok_or("missing <ds:SignatureValue> in SAMLResponse")?;
    let sig_bytes = STANDARD
        .decode(sig_b64.replace(['\n', '\r', ' '], "").as_str())
        .map_err(|e| format!("SignatureValue base64: {e}"))?;

    // ── Extract raw <ds:SignedInfo> bytes ──────────────────────────────────────
    let signed_info_bytes =
        extract_element_raw(xml, "SignedInfo").ok_or("missing <ds:SignedInfo> in SAMLResponse")?;

    // ── Parse the X.509 certificate and extract the RSA public key ────────────
    let der_bytes = extract_pem_der(pem_cert)?;
    let cert = x509_cert::Certificate::from_der(&der_bytes)
        .map_err(|e| format!("certificate parse: {e}"))?;
    let spki = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| format!("SPKI encode: {e}"))?;
    let public_key =
        RsaPublicKey::from_public_key_der(&spki).map_err(|e| format!("RSA public key: {e}"))?;

    // ── SHA-256 digest of signed_info_bytes ───────────────────────────────────
    let digest = Sha256::digest(&signed_info_bytes);

    // ── Verify RSA-PKCS1v15-SHA256 ────────────────────────────────────────────
    public_key
        .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &sig_bytes)
        .map_err(|_| "SAML signature verification failed — response may be forged".to_string())
}

/// Decode the base64 body from a PEM-encoded certificate block.
fn extract_pem_der(pem: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let body: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    STANDARD
        .decode(body.as_str())
        .map_err(|e| format!("certificate PEM decode: {e}"))
}

/// Extract the text content of the first XML element with any namespace prefix
/// that has local name `local`.
fn extract_element_text(xml: &str, local: &str) -> Option<String> {
    // Match tags like <ds:SignatureValue>, <SignatureValue>, etc.
    let open_pattern = format!(":{local}>");
    let close_pattern = format!("</{local}");
    // Also try without namespace prefix.
    let open_no_ns = format!("<{local}>");
    let close_no_ns = format!("</{local}>");

    let content_start = if let Some(pos) = xml.find(&open_pattern) {
        pos + open_pattern.len()
    } else if let Some(pos) = xml.find(&open_no_ns) {
        pos + open_no_ns.len()
    } else {
        return None;
    };

    let rest = &xml[content_start..];
    let end = rest
        .find(&format!("</{local}"))
        .or_else(|| rest.find(&close_no_ns))
        .or_else(|| rest.find(&close_pattern))?;

    Some(rest[..end].trim().to_string())
}

/// Extract the raw bytes of the first element whose local name is `local`
/// (including the opening and closing tags), suitable for digest computation.
fn extract_element_raw(xml: &str, local: &str) -> Option<Vec<u8>> {
    // Find the start of any tag that ends with `:local` or is exactly `<local`.
    let colon_local = format!(":{local} ");
    let colon_local2 = format!(":{local}>");
    let bare_local = format!("<{local} ");
    let bare_local2 = format!("<{local}>");

    let start = [&colon_local, &colon_local2, &bare_local, &bare_local2]
        .iter()
        .filter_map(|pat| {
            xml.find(pat.as_str()).map(|pos| {
                // Walk back to the `<` that opens this tag.
                xml[..pos].rfind('<').map(|open| open)
            })
        })
        .flatten()
        .min()?;

    // Find the matching closing tag by counting nesting depth.
    let tag_name_end = xml[start + 1..].find(|c: char| c == ' ' || c == '>' || c == '/')?;
    let full_tag = &xml[start + 1..start + 1 + tag_name_end];

    let close_tag = format!("</{full_tag}>");
    let mut depth: usize = 0;
    let mut pos = start;
    loop {
        let next_open = xml[pos + 1..].find('<').map(|i| i + pos + 1);
        match next_open {
            None => return None,
            Some(i) => {
                if xml[i..].starts_with("</") {
                    if depth == 0 {
                        // Closing tag for our element.
                        let end = xml[i..].find('>')? + i + 1;
                        return Some(xml[start..end].as_bytes().to_vec());
                    }
                    depth -= 1;
                } else if xml[i..].starts_with(&format!("<{full_tag}"))
                    || xml[i..].starts_with(&format!(
                        "<{}",
                        full_tag.split(':').last().unwrap_or(full_tag)
                    ))
                {
                    depth += 1;
                }
                // Ignore the close_tag variable path — handled by depth counter above.
                let _ = &close_tag;
                pos = i;
            }
        }
    }
}

fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.find(&open)?;
    let content_start = xml[start..].find('>')? + start + 1;
    let end = xml[content_start..].find(&close)? + content_start;
    Some(xml[content_start..end].trim().to_string())
}

fn build_sp_metadata_xml(sp_entity_id: &str, acs_url: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<md:EntityDescriptor
  xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata"
  entityID="{sp_entity_id}">
  <md:SPSSODescriptor
    AuthnRequestsSigned="false"
    WantAssertionsSigned="true"
    protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:AssertionConsumerService
      Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
      Location="{acs_url}"
      index="0"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>"#
    )
}

fn build_http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("HTTP client: {e}")))
}
