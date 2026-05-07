-- Audit trail enhancements: IP address, user agent, severity, user-activity index

ALTER TABLE audit_events
    ADD COLUMN IF NOT EXISTS ip_address  INET,
    ADD COLUMN IF NOT EXISTS user_agent  TEXT,
    ADD COLUMN IF NOT EXISTS severity    TEXT NOT NULL DEFAULT 'info'
        CHECK (severity IN ('info', 'warning', 'critical'));

-- Fast look-up of all events for a specific user
CREATE INDEX IF NOT EXISTS idx_audit_user
    ON audit_events (organization_id, user_id, created_at DESC);

-- Fast look-up by severity
CREATE INDEX IF NOT EXISTS idx_audit_severity
    ON audit_events (organization_id, severity, created_at DESC)
    WHERE severity <> 'info';
