-- Inventory stocktake: periodic physical count to reconcile on-hand quantities.
CREATE TABLE inventory_stocktakes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    stocktake_date  DATE NOT NULL,
    warehouse_id    UUID REFERENCES warehouses(id),
    status          TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft', 'submitted', 'posted')),
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE inventory_stocktake_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stocktake_id    UUID NOT NULL REFERENCES inventory_stocktakes(id) ON DELETE CASCADE,
    product_id      UUID NOT NULL REFERENCES products(id),
    -- Snapshot of system quantity at time of stocktake creation
    system_qty      BIGINT NOT NULL DEFAULT 0,
    counted_qty     BIGINT NOT NULL DEFAULT 0,
    variance        BIGINT GENERATED ALWAYS AS (counted_qty - system_qty) STORED,
    notes           TEXT,
    UNIQUE (stocktake_id, product_id)
);

CREATE INDEX idx_stocktakes_org ON inventory_stocktakes(organization_id, status);
