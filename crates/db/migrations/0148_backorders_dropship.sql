-- Backorders & Drop-shipping

-- Track backordered quantity per SO line
ALTER TABLE sales_order_lines
    ADD COLUMN IF NOT EXISTS quantity_backordered BIGINT NOT NULL DEFAULT 0;

-- Backorder records (created when stock can't fulfill a SO line)
CREATE TABLE backorders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    so_id           UUID NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    so_line_id      UUID NOT NULL REFERENCES sales_order_lines(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id) ON DELETE SET NULL,
    quantity        BIGINT NOT NULL CHECK (quantity > 0),
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'fulfilled', 'cancelled')),
    expected_date   DATE,
    fulfilled_at    TIMESTAMPTZ,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Drop-ship requests: vendor ships directly to customer from a SO line
CREATE TABLE drop_ship_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    so_id           UUID NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    so_line_id      UUID NOT NULL REFERENCES sales_order_lines(id) ON DELETE CASCADE,
    po_id           UUID REFERENCES purchase_orders(id) ON DELETE SET NULL,
    vendor_id       UUID NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    product_id      UUID REFERENCES products(id) ON DELETE SET NULL,
    quantity        BIGINT NOT NULL CHECK (quantity > 0),
    status          TEXT NOT NULL DEFAULT 'requested'
                        CHECK (status IN ('requested', 'po_created', 'shipped', 'delivered', 'cancelled')),
    ship_to_name    TEXT,
    ship_to_address TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_backorders_org    ON backorders(organization_id, status, created_at DESC);
CREATE INDEX idx_backorders_so     ON backorders(so_id);
CREATE INDEX idx_dropship_org      ON drop_ship_requests(organization_id, status, created_at DESC);
CREATE INDEX idx_dropship_so       ON drop_ship_requests(so_id);
