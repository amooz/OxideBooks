# API Reference

All API endpoints are under the `/api/v1` prefix unless otherwise noted.

**Base URL:** `http://localhost:3000` (configurable via `OXIDEBOOKS__APP__BASE_URL`)

**Content-Type:** `application/json` for all requests and responses.

**Authorization:** Protected endpoints require `Authorization: Bearer <jwt>` where the JWT is obtained from `/api/v1/auth/login` or `/api/v1/auth/register`.

**Response envelope:** Successful responses wrap data in `{"data": ...}`. Errors return `{"error": {"code": "...", "message": "..."}}`.

---

## Health Check

### `GET /health`

No authentication required. Returns `200 OK` with body `ok`.

---

## Authentication

### `POST /api/v1/auth/register`

Creates a new organization and its first owner user.

Only available when `OXIDEBOOKS__APP__REGISTRATION_OPEN=true` (default).

**Request:**
```json
{
  "org_name": "Acme Corp",
  "currency": "USD",
  "name": "Jane Smith",
  "email": "jane@acme.com",
  "password": "hunter2-but-longer"
}
```

`currency` is optional (defaults to `USD`). Password must be at least 12 characters.

**Response `201`:**
```json
{
  "data": {
    "token": "<jwt>",
    "user_id": "uuid",
    "org_id": "uuid",
    "role": "owner"
  }
}
```

**Errors:** `400` invalid input, `409` email already registered, `403` registration closed.

---

### `POST /api/v1/auth/login`

**Request:**
```json
{
  "org_id": "uuid",
  "email": "jane@acme.com",
  "password": "hunter2-but-longer"
}
```

**Response `200`:**
```json
{
  "data": {
    "token": "<jwt>",
    "user_id": "uuid",
    "org_id": "uuid",
    "role": "owner"
  }
}
```

**Errors:** `401` invalid credentials, `404` org or user not found.

---

### SSO — OIDC

### `GET /api/v1/auth/oidc/{provider_id}`

Initiates an OIDC authorization code + PKCE flow. Redirects the browser to the identity provider's authorization endpoint.

Query parameters:
- `post_login_uri` (optional) — where to redirect after successful login (default `/`)

**Response:** `302` redirect to IdP.

---

### `GET /api/v1/auth/oidc/{provider_id}/callback`

OIDC callback endpoint (redirect URI registered with the IdP). Exchanges the authorization code, resolves or provisions the user, and redirects to `post_login_uri` with `?token=<jwt>`.

Query parameters (set by IdP): `code`, `state`

**Response:** `302` redirect to `post_login_uri?token=<jwt>` on success.

**Errors:** `401` invalid state (expired or replayed), `500` token exchange failure.

---

### SSO — SAML 2.0

### `GET /api/v1/auth/saml/{provider_id}`

SP-initiated SAML redirect. Builds a signed `AuthnRequest`, encodes it with DEFLATE + Base64 (HTTP-Redirect binding), and redirects the browser to the IdP SSO URL.

**Response:** `302` redirect to IdP SSO URL.

---

### `POST /api/v1/auth/saml/{provider_id}/callback`

SAML ACS (Assertion Consumer Service) endpoint. Parses the `SAMLResponse`, extracts the subject and email, resolves or provisions the user, and redirects with the JWT.

Form body: `SAMLResponse=<base64>` (standard SAML HTTP-POST binding)

**Response:** `302` redirect with `?token=<jwt>`.

> **Note:** SAML response signature verification is not yet implemented. Do not expose this endpoint without an additional layer of trust (e.g., network-level controls) until signature validation is added.

---

### `GET /api/v1/auth/saml/{provider_id}/metadata`

Returns the SP metadata XML (entity ID, ACS URL, supported bindings). Feed this to your IdP during configuration.

**Response `200`:** `Content-Type: application/xml`

---

## Accounts (Chart of Accounts)

**Required permission:** `accounts:read` (GET), `accounts:write` (POST/PATCH), `accounts:delete` (DELETE)

### `GET /api/v1/accounts`

Returns all accounts for the organization, ordered by code.

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "code": "1000",
      "name": "Cash",
      "account_type": "asset",
      "parent_id": null,
      "description": null,
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

**Account types:** `asset`, `liability`, `equity`, `revenue`, `expense`

---

### `POST /api/v1/accounts`

**Request:**
```json
{
  "code": "1100",
  "name": "Accounts Receivable",
  "account_type": "asset",
  "parent_id": null,
  "description": "Amounts owed by customers"
}
```

`parent_id` and `description` are optional.

**Response `201`:** `{"data": <account>}`

**Errors:** `409` duplicate code within organization.

---

### `GET /api/v1/accounts/{id}`

**Response `200`:** `{"data": <account>}`

**Errors:** `404` not found.

---

### `PATCH /api/v1/accounts/{id}`

Partial update — any combination of fields may be omitted.

**Request:**
```json
{
  "name": "New Name",
  "code": "1101",
  "description": "Updated description",
  "is_active": false
}
```

**Response `200`:** `{"data": <updated account>}`

---

### `DELETE /api/v1/accounts/{id}`

**Response `204`** on success.

**Errors:** `404` not found.

---

## Journal Entries (Transactions)

**Required permission:** `transactions:read` (GET), `transactions:write` (POST)

Journal entries are posted immediately on creation (status = `posted`).

The double-entry invariant is enforced: Σ debits must equal Σ credits, each entry must have at least two lines, and no line may have both a debit and a credit.

### `GET /api/v1/transactions`

Returns all journal entries ordered by date descending.

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "date": "2024-01-15",
      "reference": "INV-001",
      "description": "Invoice payment received",
      "status": "posted",
      "lines": [
        {
          "id": "uuid",
          "journal_entry_id": "uuid",
          "account_id": "uuid",
          "description": "Cash receipt",
          "debit": 10000,
          "credit": 0
        },
        {
          "id": "uuid",
          "journal_entry_id": "uuid",
          "account_id": "uuid",
          "description": "Revenue recognized",
          "debit": 0,
          "credit": 10000
        }
      ],
      "created_by": "uuid",
      "created_at": "2024-01-15T12:00:00Z",
      "updated_at": "2024-01-15T12:00:00Z"
    }
  ]
}
```

Amounts are in minor units (e.g. `10000` = $100.00).

---

### `POST /api/v1/transactions`

**Request:**
```json
{
  "date": "2024-01-15",
  "reference": "INV-001",
  "description": "Invoice payment received",
  "lines": [
    { "account_id": "uuid", "description": "Cash", "debit": 10000, "credit": 0 },
    { "account_id": "uuid", "description": "Revenue", "debit": 0, "credit": 10000 }
  ]
}
```

`reference` and `description` are optional. Each line must have exactly one of `debit` or `credit` > 0.

**Response `201`:** `{"data": <journal entry with lines>}`

**Errors:** `422` double-entry validation failure (unbalanced, fewer than 2 lines, etc.)

---

### `GET /api/v1/transactions/{id}`

**Response `200`:** `{"data": <journal entry with lines>}`

---

## Contacts

**Required permission:** `contacts:read` (GET), `contacts:write` (POST/PATCH)

### `GET /api/v1/contacts`

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "name": "Acme Supplier",
      "contact_type": "vendor",
      "email": "billing@acme.com",
      "phone": "+1-555-0100",
      "address": "123 Main St",
      "tax_number": "US12-3456789",
      "currency": "USD",
      "is_active": true,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

**Contact types:** `customer`, `vendor`, `both`

---

### `POST /api/v1/contacts`

**Request:**
```json
{
  "name": "Acme Supplier",
  "contact_type": "vendor",
  "email": "billing@acme.com",
  "phone": "+1-555-0100",
  "address": "123 Main St",
  "tax_number": "US12-3456789",
  "currency": "USD"
}
```

All fields except `name` are optional. `contact_type` defaults to `both`.

**Response `201`:** `{"data": <contact>}`

---

### `GET /api/v1/contacts/{id}`

**Response `200`:** `{"data": <contact>}`

---

### `PATCH /api/v1/contacts/{id}`

Any combination of fields may be omitted:

**Request:**
```json
{
  "name": "New Name",
  "email": "new@email.com",
  "phone": null,
  "address": null,
  "tax_number": null,
  "currency": "EUR",
  "is_active": true
}
```

**Response `200`:** `{"data": <updated contact>}`

---

## Invoices & Bills

**Required permission:** `invoices:read` (GET), `invoices:write` (POST)

**Invoice types:** `invoice` (accounts receivable) / `bill` (accounts payable)

Invoice numbers are auto-generated as `INV-00001` / `BILL-00001` (sequential per organization per type).

### `GET /api/v1/invoices`

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "organization_id": "uuid",
      "invoice_number": "INV-00001",
      "contact_id": "uuid",
      "invoice_type": "invoice",
      "status": "draft",
      "date": "2024-01-15",
      "due_date": "2024-02-15",
      "currency": "USD",
      "notes": null,
      "lines": [
        {
          "id": "uuid",
          "invoice_id": "uuid",
          "description": "Consulting services",
          "account_id": "uuid",
          "quantity": 200,
          "unit_price": 15000,
          "tax_rate": 1000,
          "sort_order": 0
        }
      ],
      "journal_entry_id": null,
      "created_at": "2024-01-15T00:00:00Z",
      "updated_at": "2024-01-15T00:00:00Z"
    }
  ]
}
```

`quantity` is ×100 (200 = 2.00 units), `unit_price` is in minor units, `tax_rate` is ×100 (1000 = 10%).

**Invoice statuses:** `draft`, `sent`, `partial`, `paid`, `overdue`, `voided`

---

### `POST /api/v1/invoices`

**Request:**
```json
{
  "contact_id": "uuid",
  "invoice_type": "invoice",
  "date": "2024-01-15",
  "due_date": "2024-02-15",
  "currency": "USD",
  "notes": "Net 30",
  "lines": [
    {
      "description": "Consulting services",
      "account_id": "uuid",
      "quantity": 200,
      "unit_price": 15000,
      "tax_rate": 1000
    }
  ]
}
```

`currency` defaults to the organization's default currency. `account_id` and `tax_rate` are optional per line. `notes` is optional.

**Response `201`:** `{"data": <invoice with lines>}`

**Errors:** `422` if any quantity ≤ 0 or unit_price < 0.

---

### `GET /api/v1/invoices/{id}`

**Response `200`:** `{"data": <invoice with lines>}`

---

## Reports

**Required permission:** `reports:read`

### `GET /api/v1/reports/trial-balance`

Returns a trial balance across all active accounts, aggregating only posted journal entries.

**Response `200`:**
```json
{
  "data": {
    "accounts": [
      {
        "account_id": "uuid",
        "account_code": "1000",
        "account_name": "Cash",
        "account_type": "asset",
        "debit_total": 500000,
        "credit_total": 100000
      }
    ],
    "total_debits": 600000,
    "total_credits": 600000,
    "is_balanced": true
  }
}
```

`balance()` per account = debit_total − credit_total for debit-normal accounts (asset, expense); credit_total − debit_total for credit-normal accounts (liability, equity, revenue).

---

## Roles & Permissions

**Required permission:** `roles:read` (GET), `roles:write` (POST/DELETE)

### `GET /api/v1/permissions`

Returns all system-defined permission strings.

**Response `200`:**
```json
{
  "data": [
    { "id": "uuid", "name": "accounts:read", "description": "View accounts" },
    { "id": "uuid", "name": "accounts:write", "description": "Create and update accounts" }
  ]
}
```

Full permission list: `accounts:{read,write,delete}`, `contacts:{read,write}`, `invoices:{read,write}`, `transactions:{read,write}`, `reports:read`, `users:{read,write,delete}`, `roles:{read,write}`

---

### `GET /api/v1/roles`

Returns system roles (visible to all orgs) and org-custom roles.

**Response `200`:**
```json
{
  "data": [
    {
      "id": "00000000-0000-0000-0000-000000000001",
      "org_id": null,
      "name": "owner",
      "is_system": true,
      "permissions": ["accounts:read", "accounts:write", "..."],
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}
```

`org_id` is `null` for system roles, a UUID for org-custom roles.

---

### `POST /api/v1/roles`

Creates an org-custom role with no permissions (assign permissions separately).

**Request:**
```json
{ "name": "billing-manager" }
```

**Response `201`:** `{"data": <role>}`

---

### `POST /api/v1/roles/{role_id}/permissions`

Assigns a permission to a role. Idempotent.

**Request:**
```json
{ "permission": "invoices:write" }
```

**Response `204`**

**Errors:** `404` role or permission not found, `403` role does not belong to the org.

---

### `DELETE /api/v1/roles/{role_id}/permissions/{permission}`

Removes a permission from a role.

**Response `204`**

---

## Identity Providers

**Required permission:** `users:read` (GET), `users:write` (POST/DELETE)

### `GET /api/v1/identity-providers`

Lists all configured identity providers for the org.

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "org_id": "uuid",
      "name": "Google Workspace",
      "provider_type": "oidc",
      "is_enabled": true,
      "email_domains": ["acme.com"],
      "oidc_client_id": "client-id",
      "oidc_issuer_url": "https://accounts.google.com",
      "oidc_scopes": "openid email profile",
      "saml_idp_metadata_url": null,
      "saml_idp_entity_id": null,
      "saml_idp_sso_url": null,
      "saml_sp_entity_id": null,
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}
```

`oidc_client_secret` and `saml_idp_certificate` are never returned by the API.

---

### `POST /api/v1/identity-providers/oidc`

**Request:**
```json
{
  "name": "Google Workspace",
  "client_id": "123456.apps.googleusercontent.com",
  "client_secret": "GOCSPX-...",
  "issuer_url": "https://accounts.google.com",
  "scopes": "openid email profile",
  "email_domains": ["acme.com"]
}
```

`scopes` defaults to `openid email profile`. `email_domains` is optional.

**Response `201`:** `{"data": <provider>}`

---

### `POST /api/v1/identity-providers/saml`

**Request:**
```json
{
  "name": "Okta",
  "idp_metadata_url": "https://acme.okta.com/app/metadata",
  "idp_entity_id": "http://www.okta.com/exk...",
  "idp_sso_url": "https://acme.okta.com/app/oxidebooks/sso/saml",
  "idp_certificate": "-----BEGIN CERTIFICATE-----\n...",
  "sp_entity_id": "https://api.acme.com/api/v1/auth/saml/uuid",
  "email_domains": ["acme.com"]
}
```

All fields are optional; provide whichever the IdP requires.

**Response `201`:** `{"data": <provider>}`

---

### `DELETE /api/v1/identity-providers/{id}`

**Response `204`**

---

## SCIM Token Management

**Required permission:** `users:read` (GET), `users:write` (POST/DELETE)

SCIM tokens are Argon2-hashed bearer tokens used to authenticate SCIM provisioning calls. The raw token is shown **once** at creation; it cannot be retrieved afterward.

### `GET /api/v1/scim/tokens`

**Response `200`:**
```json
{
  "data": [
    {
      "id": "uuid",
      "org_id": "uuid",
      "name": "Okta Provisioner",
      "is_active": true,
      "last_used_at": "2024-01-15T12:00:00Z",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

---

### `POST /api/v1/scim/tokens`

**Request:**
```json
{ "name": "Okta Provisioner" }
```

**Response `201`:**
```json
{
  "data": {
    "id": "uuid",
    "org_id": "uuid",
    "name": "Okta Provisioner",
    "is_active": true,
    "last_used_at": null,
    "created_at": "2024-01-01T00:00:00Z",
    "raw_token": "scim_abc123..."
  }
}
```

Save `raw_token` immediately — it is not stored and cannot be retrieved again.

---

### `DELETE /api/v1/scim/tokens/{id}`

Revokes the token (sets `is_active = false`).

**Response `204`**

---

## SCIM 2.0 Endpoints

SCIM endpoints use **separate bearer-token auth** (not JWT). Include the raw token from `POST /api/v1/scim/tokens` as `Authorization: Bearer scim_...`.

Base path: `/scim/v2/` (not under `/api/v1`).

### `GET /scim/v2/ServiceProviderConfig`

Returns supported SCIM 2.0 features.

### `GET /scim/v2/Users`

Lists provisioned users for the token's organization.

**Response:**
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
  "totalResults": 1,
  "startIndex": 1,
  "itemsPerPage": 1,
  "Resources": [
    {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
      "id": "uuid",
      "userName": "jane@acme.com",
      "displayName": "Jane Smith",
      "active": true,
      "emails": [{ "value": "jane@acme.com", "primary": true }],
      "roles": [{ "value": "viewer" }],
      "meta": { "resourceType": "User", "location": "/scim/v2/Users/uuid" }
    }
  ]
}
```

### `POST /scim/v2/Users`

Provisions a new user with the `viewer` role.

**Request:**
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "jane@acme.com",
  "displayName": "Jane Smith",
  "active": true,
  "name": { "givenName": "Jane", "familyName": "Smith" },
  "emails": [{ "value": "jane@acme.com", "primary": true }]
}
```

**Response `201`:** SCIM user resource.

### `GET /scim/v2/Users/{id}`

**Response `200`:** SCIM user resource.

### `PATCH /scim/v2/Users/{id}`

Supports `replace` operation. Primary use: de-provisioning by setting `active = false`.

**Request:**
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
  "Operations": [
    { "op": "replace", "path": "active", "value": false }
  ]
}
```

**Response `200`:** Updated SCIM user resource.

### `DELETE /scim/v2/Users/{id}`

Deactivates the user (`is_active = false`). Does not hard-delete.

**Response `204`**

---

## Error Response Format

All errors follow this shape:

```json
{
  "error": {
    "code": "not_found",
    "message": "resource not found"
  }
}
```

| HTTP Status | Code | Cause |
|---|---|---|
| 400 | `bad_request` | Malformed request |
| 401 | `unauthorized` | Missing or invalid JWT / SCIM token |
| 403 | `forbidden` | Valid token but insufficient permissions |
| 404 | `not_found` | Resource does not exist (or belongs to another org) |
| 409 | `conflict` | Unique constraint violation (duplicate code, email, etc.) |
| 422 | `validation_error` | Domain validation failure (unbalanced entry, negative amount, etc.) |
| 500 | `internal_error` | Unexpected server error (details logged server-side only) |
