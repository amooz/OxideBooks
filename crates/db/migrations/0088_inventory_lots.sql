CREATE TABLE inventory_lots (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    item_id          UUID        NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    lot_number       TEXT        NOT NULL,
    expiry_date      DATE,
    quantity         BIGINT      NOT NULL DEFAULT 0,
    cost_per_unit    BIGINT      NOT NULL DEFAULT 0,
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (item_id, lot_number)
);

CREATE INDEX ON inventory_lots (organization_id);
CREATE INDEX ON inventory_lots (item_id);
CREATE INDEX ON inventory_lots (expiry_date) WHERE expiry_date IS NOT NULL;
