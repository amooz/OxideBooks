CREATE TABLE project_tasks (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      UUID        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_id        UUID        REFERENCES project_phases(id) ON DELETE SET NULL,
    name            TEXT        NOT NULL,
    description     TEXT,
    assignee_id     UUID        REFERENCES users(id) ON DELETE SET NULL,
    status          TEXT        NOT NULL DEFAULT 'open'
                                CHECK (status IN ('open','in_progress','completed','cancelled')),
    due_date        DATE,
    estimated_minutes INT       CHECK (estimated_minutes > 0),
    actual_minutes  INT         NOT NULL DEFAULT 0 CHECK (actual_minutes >= 0),
    sort_order      INT         NOT NULL DEFAULT 0,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_project_tasks_project ON project_tasks (organization_id, project_id, sort_order);
CREATE INDEX idx_project_tasks_assignee ON project_tasks (organization_id, assignee_id);
