CREATE TABLE notes (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID    NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id     UUID        REFERENCES users(id) ON DELETE SET NULL,
    entity_type TEXT        NOT NULL,
    entity_id   UUID        NOT NULL,
    body        TEXT        NOT NULL,
    is_system   BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notes_entity ON notes (entity_id, created_at DESC);
CREATE INDEX idx_notes_org    ON notes (organization_id);
