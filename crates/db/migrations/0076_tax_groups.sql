-- Tax groups: combine multiple tax rates into a single named group
CREATE TABLE tax_groups (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE tax_group_rates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID NOT NULL REFERENCES tax_groups(id) ON DELETE CASCADE,
    tax_rate_id UUID NOT NULL REFERENCES tax_rates(id) ON DELETE RESTRICT,
    sort_order  INT NOT NULL DEFAULT 0,
    UNIQUE (group_id, tax_rate_id)
);

CREATE INDEX idx_tax_groups_org  ON tax_groups(organization_id);
CREATE INDEX idx_tgr_group       ON tax_group_rates(group_id);
