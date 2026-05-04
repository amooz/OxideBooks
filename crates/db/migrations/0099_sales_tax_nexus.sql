CREATE TABLE sales_tax_nexus (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    jurisdiction_code   TEXT        NOT NULL,
    jurisdiction_name   TEXT        NOT NULL,
    nexus_type          TEXT        NOT NULL DEFAULT 'physical'
                            CHECK (nexus_type IN ('physical', 'economic')),
    registration_number TEXT,
    effective_date      DATE        NOT NULL,
    end_date            DATE,
    status              TEXT        NOT NULL DEFAULT 'active'
                            CHECK (status IN ('active', 'inactive')),
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, jurisdiction_code)
);
