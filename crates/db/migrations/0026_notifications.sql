CREATE TABLE notifications (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notification_type TEXT      NOT NULL,
    title           TEXT        NOT NULL,
    body            TEXT        NOT NULL,
    entity_type     TEXT,
    entity_id       UUID,
    is_read         BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications (organization_id, user_id, is_read, created_at DESC);
