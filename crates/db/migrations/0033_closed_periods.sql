CREATE TABLE closed_periods (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    closed_by       UUID REFERENCES users(id) ON DELETE SET NULL,
    closed_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    notes           TEXT,
    CONSTRAINT no_overlap EXCLUDE USING gist (
        organization_id WITH =,
        daterange(period_start, period_end, '[]') WITH &&
    )
);

CREATE INDEX idx_closed_periods_org ON closed_periods(organization_id, period_start DESC);
