-- Identity providers (OIDC and SAML), SCIM tokens, OIDC state store,
-- and auth method tracking on users.

-- ─── Identity Providers ───────────────────────────────────────────────────────

CREATE TABLE identity_providers (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id           UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    -- 'oidc' | 'saml'
    provider_type    TEXT NOT NULL CHECK (provider_type IN ('oidc', 'saml')),
    is_enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    -- Email domains that should be redirected to this IdP (e.g. '{example.com}')
    email_domains    TEXT[] NOT NULL DEFAULT '{}',

    -- ── OIDC fields ──
    oidc_client_id       TEXT,
    oidc_client_secret   TEXT,  -- store encrypted in production
    oidc_issuer_url      TEXT,
    oidc_scopes          TEXT NOT NULL DEFAULT 'openid email profile',

    -- ── SAML fields ──
    saml_idp_metadata_url TEXT,
    saml_idp_entity_id    TEXT,
    saml_idp_sso_url      TEXT,
    saml_idp_certificate  TEXT,  -- PEM-encoded X.509 cert
    saml_sp_entity_id     TEXT,

    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, name)
);

CREATE INDEX idx_idp_org ON identity_providers(org_id);

-- ─── Extend users table for SSO ───────────────────────────────────────────────

ALTER TABLE users
    ADD COLUMN auth_method          TEXT NOT NULL DEFAULT 'local'
        CHECK (auth_method IN ('local', 'oidc', 'saml', 'scim')),
    ADD COLUMN identity_provider_id UUID REFERENCES identity_providers(id),
    ADD COLUMN external_id          TEXT;  -- subject from the IdP

-- Ensure (identity_provider_id, external_id) is unique when set.
CREATE UNIQUE INDEX idx_users_external
    ON users(identity_provider_id, external_id)
    WHERE external_id IS NOT NULL;

-- ─── OIDC State store (anti-CSRF) ────────────────────────────────────────────

CREATE TABLE oidc_states (
    state           TEXT PRIMARY KEY,
    provider_id     UUID NOT NULL REFERENCES identity_providers(id) ON DELETE CASCADE,
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- PKCE code verifier (S256 challenge)
    code_verifier   TEXT,
    -- Where to send the user after successful authentication
    post_login_uri  TEXT NOT NULL DEFAULT '/',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- States expire after 15 minutes; clean up with a cron job or on each request.
CREATE INDEX idx_oidc_states_created ON oidc_states(created_at);

-- ─── SCIM provisioning tokens ─────────────────────────────────────────────────

CREATE TABLE scim_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id       UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    token_hash   TEXT NOT NULL,  -- Argon2 hash of the raw bearer token
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, name)
);

CREATE INDEX idx_scim_tokens_org ON scim_tokens(org_id);
