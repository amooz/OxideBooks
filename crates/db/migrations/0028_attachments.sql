CREATE TABLE attachments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    entity_type     TEXT NOT NULL,
    entity_id       UUID NOT NULL,
    file_name       TEXT NOT NULL,
    file_size       BIGINT NOT NULL DEFAULT 0,
    content_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
    storage_url     TEXT NOT NULL,
    uploaded_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_attachments_entity ON attachments(organization_id, entity_type, entity_id);
