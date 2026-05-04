CREATE TABLE project_phases (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      UUID        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    description     TEXT,
    budget          BIGINT      NOT NULL DEFAULT 0 CHECK (budget >= 0),
    start_date      DATE,
    end_date        DATE,
    status          TEXT        NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'completed', 'cancelled')),
    sort_order      INT         NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_project_phases_project ON project_phases(project_id);

-- Allow time entries and expenses to be linked to a project phase.
ALTER TABLE time_entries ADD COLUMN phase_id UUID REFERENCES project_phases(id);
ALTER TABLE expenses     ADD COLUMN phase_id UUID REFERENCES project_phases(id);
