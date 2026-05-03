CREATE TABLE projects (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    contact_id      UUID        REFERENCES contacts(id) ON DELETE SET NULL,
    status          TEXT        NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','completed','archived')),
    budget_amount   BIGINT,
    start_date      DATE,
    end_date        DATE,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_projects_org ON projects (organization_id);

CREATE TABLE time_entries (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    project_id      UUID        REFERENCES projects(id) ON DELETE SET NULL,
    contact_id      UUID        REFERENCES contacts(id) ON DELETE SET NULL,
    entry_date      DATE        NOT NULL,
    minutes         INT         NOT NULL CHECK (minutes > 0),
    description     TEXT        NOT NULL,
    hourly_rate     BIGINT      NOT NULL DEFAULT 0,
    is_billable     BOOLEAN     NOT NULL DEFAULT TRUE,
    invoice_line_id UUID        REFERENCES invoice_lines(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_time_entries_org    ON time_entries (organization_id, entry_date DESC);
CREATE INDEX idx_time_entries_user   ON time_entries (organization_id, user_id);
CREATE INDEX idx_time_entries_proj   ON time_entries (project_id);
