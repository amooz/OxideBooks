-- Tracks reorder requests generated from low-stock alerts.
-- Each request maps to a draft purchase order once submitted.
CREATE TABLE inventory_reorder_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    product_id      UUID NOT NULL REFERENCES products(id),
    supplier_id     UUID REFERENCES contacts(id),
    requested_qty   BIGINT NOT NULL CHECK (requested_qty > 0),
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','ordered','cancelled')),
    purchase_order_id UUID REFERENCES purchase_orders(id),
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_reorder_requests_org ON inventory_reorder_requests(organization_id, status);
