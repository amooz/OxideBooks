CREATE TABLE sales_order_shipments (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    sales_order_id  UUID        NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    shipped_at      DATE        NOT NULL DEFAULT CURRENT_DATE,
    tracking_number TEXT,
    carrier         TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sales_order_shipment_lines (
    id               UUID       PRIMARY KEY DEFAULT gen_random_uuid(),
    shipment_id      UUID       NOT NULL REFERENCES sales_order_shipments(id) ON DELETE CASCADE,
    so_line_id       UUID       NOT NULL REFERENCES sales_order_lines(id),
    product_id       UUID       REFERENCES products(id),
    quantity_shipped BIGINT     NOT NULL CHECK (quantity_shipped > 0)
);

CREATE INDEX idx_so_shipments_org ON sales_order_shipments (organization_id, sales_order_id);
