-- Tax rules: map a jurisdiction/region to a default tax rate.
-- Used to auto-suggest or auto-apply a tax rate when creating invoices/bills
-- for contacts in a given region.
CREATE TABLE tax_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    country_code    TEXT NOT NULL,
    region_code     TEXT,
    tax_rate_id     UUID NOT NULL REFERENCES tax_rates(id) ON DELETE CASCADE,
    applies_to      TEXT NOT NULL DEFAULT 'sales'
                        CHECK (applies_to IN ('sales', 'purchases', 'both')),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    priority        INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, country_code, region_code, applies_to)
);

CREATE INDEX idx_tax_rules_org ON tax_rules(organization_id, is_active, applies_to);
