CREATE TABLE client_portal_tokens (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    token           TEXT        NOT NULL UNIQUE,
    contact_id      UUID        NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON client_portal_tokens(token);
