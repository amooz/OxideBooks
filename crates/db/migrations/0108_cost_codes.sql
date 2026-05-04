-- Cost codes for job costing within projects/phases
CREATE TABLE cost_codes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    code            TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    cost_type       TEXT NOT NULL DEFAULT 'labor'
                        CHECK (cost_type IN ('labor','material','equipment','subcontractor','overhead','other')),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, code)
);

-- Link cost codes to time entries and expenses for job costing
ALTER TABLE time_entries ADD COLUMN cost_code_id UUID REFERENCES cost_codes(id);
ALTER TABLE expenses     ADD COLUMN cost_code_id UUID REFERENCES cost_codes(id);

CREATE INDEX idx_cost_codes_org ON cost_codes(organization_id, is_active);
