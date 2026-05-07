CREATE TYPE report_schedule_frequency AS ENUM ('daily', 'weekly', 'monthly', 'quarterly');

CREATE TABLE report_schedules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    report_type     TEXT NOT NULL,
    frequency       report_schedule_frequency NOT NULL,
    params          JSONB NOT NULL DEFAULT '{}',
    recipients      TEXT[] NOT NULL DEFAULT '{}',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at     TIMESTAMPTZ,
    next_run_at     TIMESTAMPTZ,
    created_by      UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON report_schedules(organization_id);
CREATE INDEX ON report_schedules(next_run_at) WHERE is_active = TRUE;
