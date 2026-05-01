# Authentication & Authorization

OxideBooks supports four authentication methods:

| Method | Use case |
|---|---|
| Local (email/password) | Human users, development, small teams |
| OIDC | SSO via Google Workspace, Okta, Azure AD, Auth0, etc. |
| SAML 2.0 | SSO via Okta, ADFS, OneLogin, and other enterprise IdPs |
| SCIM 2.0 | Machine-to-machine automated user provisioning |

All methods result in a JWT that is used for subsequent API calls. SCIM provisioning uses a separate bearer token, not a JWT.

---

## Local Authentication

### Registration

`POST /api/v1/auth/register` creates an organization and its first user with the **owner** role.

- Password is hashed with Argon2 before storage. The plaintext is never persisted.
- Minimum password length: 12 characters.
- Registration can be disabled by setting `OXIDEBOOKS__APP__REGISTRATION_OPEN=false` (useful after initial setup).

### Login

`POST /api/v1/auth/login` verifies the Argon2 password hash and issues a JWT.

The JWT payload (`Claims`) contains:

```json
{
  "sub": "<user_id>",
  "org": "<org_id>",
  "role": "owner",
  "permissions": ["accounts:read", "accounts:write", "..."],
  "exp": 1700000000
}
```

`permissions` is the full list of permission strings granted to the user via their role. Permission checks in handlers use `claims.has("resource:action")` — a simple exact-match against this list.

**JWT settings:**
- Algorithm: HS256
- Secret: `OXIDEBOOKS__AUTH__JWT_SECRET` (must be ≥ 32 characters)
- Default expiry: 24 hours (`OXIDEBOOKS__AUTH__TOKEN_EXPIRY_HOURS`)

---

## OIDC (OpenID Connect)

### Overview

OxideBooks implements the Authorization Code flow with PKCE (RFC 7636). The flow is:

```
Browser                OxideBooks API            Identity Provider
   |                        |                           |
   |  GET /auth/oidc/:id    |                           |
   |----------------------->|                           |
   |                        |  discover metadata        |
   |                        |  generate state + PKCE    |
   |                        |  store state in oidc_states
   |  302 → IdP auth URL    |                           |
   |<-----------------------|                           |
   |                                                    |
   |  user authenticates at IdP                         |
   |                                                    |
   |  GET /auth/oidc/:id/callback?code=…&state=…        |
   |----------------------->|                           |
   |                        |  consume & verify state   |
   |                        |  exchange code for tokens |
   |                        |-------------------------->|
   |                        |<--------------------------|
   |                        |  extract email + subject  |
   |                        |  upsert user              |
   |                        |  issue JWT                |
   |  302 → post_login_uri?token=…                      |
   |<-----------------------|                           |
```

### Setup

1. **Register the callback URL** with your IdP:
   ```
   {BASE_URL}/api/v1/auth/oidc/{provider_id}/callback
   ```
   where `BASE_URL` is `OXIDEBOOKS__APP__BASE_URL` (e.g. `https://api.acme.com`).

2. **Create the provider** via API:
   ```http
   POST /api/v1/identity-providers/oidc
   Authorization: Bearer <admin-jwt>

   {
     "name": "Google Workspace",
     "client_id": "123456.apps.googleusercontent.com",
     "client_secret": "GOCSPX-...",
     "issuer_url": "https://accounts.google.com",
     "email_domains": ["acme.com"]
   }
   ```
   Note the returned `id` — this is the `{provider_id}` used in URLs.

3. **Initiate login** by redirecting the user to:
   ```
   GET /api/v1/auth/oidc/{provider_id}?post_login_uri=/dashboard
   ```

### Anti-CSRF & PKCE

State is stored in the `oidc_states` table and deleted on consumption (one-time use). States expire after 15 minutes. PKCE code_verifier is included in the state so no client-side storage is required.

### User Provisioning

On successful OIDC callback, users are resolved in this order:

1. **Existing SSO user** — find by `(identity_provider_id, external_id)` (the IdP's subject claim)
2. **Existing local user** — find by email within the org; link the SSO identity
3. **Auto-provision** — create a new user with the **viewer** role

---

## SAML 2.0

### Overview

OxideBooks acts as the Service Provider (SP) in an SP-initiated SSO flow using HTTP-Redirect binding for the `AuthnRequest` and HTTP-POST binding for the `SAMLResponse`.

```
Browser                OxideBooks (SP)           Identity Provider (IdP)
   |                        |                           |
   |  GET /auth/saml/:id    |                           |
   |----------------------->|                           |
   |                        |  build AuthnRequest XML   |
   |                        |  DEFLATE + Base64 encode  |
   |  302 → IdP SSO URL?SAMLRequest=…                  |
   |<-----------------------|                           |
   |                                                    |
   |  user authenticates at IdP                         |
   |                                                    |
   |  POST /auth/saml/:id/callback                      |
   |  (SAMLResponse=<base64>)                           |
   |----------------------->|                           |
   |                        |  decode + parse response  |
   |                        |  extract NameID + email   |
   |                        |  upsert user              |
   |                        |  issue JWT                |
   |  302 → ?token=…        |                           |
   |<-----------------------|                           |
```

### Setup

1. **Get the SP metadata** (used to configure your IdP):
   ```
   GET /api/v1/auth/saml/{provider_id}/metadata
   ```
   Download or paste this XML into your IdP's application configuration.

2. **Create the provider**:
   ```http
   POST /api/v1/identity-providers/saml
   Authorization: Bearer <admin-jwt>

   {
     "name": "Okta",
     "idp_sso_url": "https://acme.okta.com/app/oxidebooks/sso/saml",
     "idp_entity_id": "http://www.okta.com/exk...",
     "idp_certificate": "-----BEGIN CERTIFICATE-----\n..."
   }
   ```

3. **ACS URL** to register with your IdP:
   ```
   {BASE_URL}/api/v1/auth/saml/{provider_id}/callback
   ```

> **Security note:** SAML response signature verification is not yet implemented. Until it is, SAML should only be used in environments where the network path from the IdP to the ACS endpoint can be trusted, or with an additional reverse-proxy layer that validates signatures.

---

## SCIM 2.0 Provisioning

SCIM (System for Cross-domain Identity Management, RFC 7644) allows identity providers like Okta, Azure AD, and JumpCloud to automatically create, update, and deactivate users in OxideBooks without manual administration.

### Authentication

SCIM uses its own bearer tokens — separate from the JWT used by human users. These tokens are scoped to an organization and are Argon2-hashed before storage.

**Create a SCIM token:**
```http
POST /api/v1/scim/tokens
Authorization: Bearer <admin-jwt>

{ "name": "Okta Provisioner" }
```

Response includes `raw_token: "scim_..."`. Copy it immediately — it cannot be retrieved again.

**Use the token for SCIM calls:**
```
Authorization: Bearer scim_abc123...
```

### Configuring Okta (example)

1. In Okta Admin → Applications → your app → Provisioning tab:
   - **SCIM connector base URL:** `https://api.acme.com/scim/v2`
   - **Unique identifier field for users:** `userName`
   - **Authentication mode:** HTTP Header
   - **Authorization:** `Bearer scim_abc123...`

2. Enable **Push Users** and **Push Profile Updates**.

3. Okta will call `POST /scim/v2/Users` for new assignments and `PATCH /scim/v2/Users/{id}` with `active: false` to deprovision.

### Provisioning Behavior

| Operation | OxideBooks behavior |
|---|---|
| Create user | Creates account with **viewer** role; email as username |
| Update user | Not yet implemented (PATCH only handles `active` field) |
| Deactivate user | Sets `is_active = false`; login disabled immediately |
| Delete user | Sets `is_active = false` (soft delete, data preserved) |

Users provisioned via SCIM authenticate via OIDC or SAML (they have no local password by default, unless one is set separately).

---

## Token Lifecycle

```
Login / SSO callback
    │
    ├─ PermissionRepo::list_for_user(user_id)   [DB lookup]
    │
    └─ JWT signed with HS256
           sub = user_id
           org = org_id
           role = role_name        (display only)
           permissions = [...]     (embedded, no per-request DB lookup)
           exp = now + token_expiry_hours
```

Tokens are stateless — revocation is not supported. When a user's role or permissions change, they must log in again to get an updated token.

For SCIM tokens, revocation is supported via `DELETE /api/v1/scim/tokens/{id}`, which sets `is_active = false` and causes the next SCIM call with that token to be rejected.
