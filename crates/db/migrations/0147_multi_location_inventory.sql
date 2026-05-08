-- Multi-location inventory: transfer status lifecycle + stock adjustment audit trail

-- Add status to inventory_transfers
-- Existing rows are immediate/completed transfers.
ALTER TABLE inventory_transfers
    ADD COLUMN status TEXT NOT NULL DEFAULT 'completed'
        CHECK (status IN ('pending', 'completed', 'cancelled'));

CREATE INDEX idx_transfer_status ON inventory_transfers(organization_id, status, created_at DESC);

-- Manual stock adjustments (cycle counts, write-offs, corrections)
CREATE TABLE stock_adjustments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    warehouse_id    UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    item_id         UUID NOT NULL REFERENCES inventory_items(id) ON DELETE CASCADE,
    quantity_delta  BIGINT NOT NULL,   -- positive = increase, negative = decrease
    reason          TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stock_adj_org ON stock_adjustments(organization_id, created_at DESC);
CREATE INDEX idx_stock_adj_wh  ON stock_adjustments(warehouse_id, created_at DESC);
