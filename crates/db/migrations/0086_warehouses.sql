-- Warehouses / stock locations for multi-location inventory management.
CREATE TABLE warehouses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    code            TEXT,
    address         TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

-- Per-warehouse stock level for each inventory item.
CREATE TABLE warehouse_stock (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id    UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    item_id         UUID NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    quantity        BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (warehouse_id, item_id)
);

-- Transfer of stock from one warehouse to another.
CREATE TABLE inventory_transfers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    from_warehouse_id   UUID NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    to_warehouse_id     UUID NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    item_id             UUID NOT NULL REFERENCES inventory_items(id) ON DELETE RESTRICT,
    quantity            BIGINT NOT NULL CHECK (quantity > 0),
    notes               TEXT,
    transferred_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wh_org   ON warehouses(organization_id);
CREATE INDEX idx_whstock_wh ON warehouse_stock(warehouse_id);
CREATE INDEX idx_transfer_org ON inventory_transfers(organization_id);
