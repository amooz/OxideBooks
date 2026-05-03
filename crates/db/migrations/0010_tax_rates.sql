CREATE TABLE tax_rates (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    rate_bps        INT         NOT NULL CHECK (rate_bps >= 0 AND rate_bps <= 100000),
    tax_type        TEXT        NOT NULL DEFAULT 'exclusive' CHECK (tax_type IN ('inclusive', 'exclusive')),
    is_default      BOOLEAN     NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tax_rates_org ON tax_rates (organization_id);

-- Only one default per org
CREATE UNIQUE INDEX idx_tax_rates_default ON tax_rates (organization_id) WHERE is_default = TRUE;
