CREATE TABLE tax_periods (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT        NOT NULL,
    period_start     DATE        NOT NULL,
    period_end       DATE        NOT NULL,
    tax_collected    BIGINT      NOT NULL DEFAULT 0,
    tax_paid         BIGINT      NOT NULL DEFAULT 0,
    net_tax          BIGINT      NOT NULL DEFAULT 0,
    status           TEXT        NOT NULL DEFAULT 'open'
                                 CHECK (status IN ('open', 'filed', 'locked')),
    filed_at         TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (period_end >= period_start)
);

CREATE INDEX tax_periods_org    ON tax_periods (organization_id);
CREATE INDEX tax_periods_status ON tax_periods (organization_id, status);
