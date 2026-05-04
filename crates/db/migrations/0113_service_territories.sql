-- Service territories: geographic zones used to assign work orders to field teams.
CREATE TABLE service_territories (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    region_code     TEXT,
    country_code    TEXT NOT NULL DEFAULT 'US',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE INDEX idx_service_territories_org ON service_territories(organization_id, is_active);

-- Link work orders to a territory for scheduling/dispatch.
ALTER TABLE work_orders ADD COLUMN territory_id UUID REFERENCES service_territories(id);
