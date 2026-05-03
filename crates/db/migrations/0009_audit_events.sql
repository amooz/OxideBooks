CREATE TABLE audit_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID        REFERENCES users(id) ON DELETE SET NULL,
    action          TEXT        NOT NULL,   -- 'create' | 'update' | 'delete'
    resource_type   TEXT        NOT NULL,   -- 'invoice' | 'transaction' | 'contact' | ...
    resource_id     TEXT        NOT NULL,
    changes         JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_org_time     ON audit_events (organization_id, created_at DESC);
CREATE INDEX idx_audit_resource     ON audit_events (organization_id, resource_type, resource_id);
