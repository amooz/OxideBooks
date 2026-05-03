CREATE TABLE email_settings (
    organization_id UUID    PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    smtp_host       TEXT    NOT NULL,
    smtp_port       INT     NOT NULL DEFAULT 587,
    smtp_user       TEXT    NOT NULL,
    smtp_password   TEXT    NOT NULL,
    from_address    TEXT    NOT NULL,
    from_name       TEXT    NOT NULL DEFAULT 'OxideBooks',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE email_log (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    to_address      TEXT        NOT NULL,
    subject         TEXT        NOT NULL,
    entity_type     TEXT,
    entity_id       UUID,
    status          TEXT        NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued','sent','failed')),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_log_org ON email_log (organization_id, created_at DESC);
