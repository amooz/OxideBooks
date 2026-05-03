CREATE TABLE inventory_items (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    product_id          UUID        NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    quantity_on_hand    BIGINT      NOT NULL DEFAULT 0,
    reorder_point       BIGINT      NOT NULL DEFAULT 0,
    cost_per_unit       BIGINT      NOT NULL DEFAULT 0,
    valuation_method    TEXT        NOT NULL DEFAULT 'average'
                        CHECK (valuation_method IN ('fifo','average')),
    UNIQUE (organization_id, product_id)
);

CREATE TABLE inventory_movements (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    item_id         UUID        NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    movement_type   TEXT        NOT NULL CHECK (movement_type IN ('purchase','sale','adjustment','return')),
    quantity        BIGINT      NOT NULL,
    unit_cost       BIGINT      NOT NULL DEFAULT 0,
    reference_id    UUID,
    reference_type  TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_inventory_items_org     ON inventory_items (organization_id);
CREATE INDEX idx_inventory_movements_item ON inventory_movements (item_id, created_at DESC);
