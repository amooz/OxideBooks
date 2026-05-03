CREATE TABLE purchase_orders (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    po_number       TEXT        NOT NULL,
    contact_id      UUID        NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    status          TEXT        NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft','sent','partially_received','received','billed','voided')),
    order_date      DATE        NOT NULL,
    expected_date   DATE,
    currency        TEXT        NOT NULL DEFAULT 'USD',
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE po_counters (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    next_val        BIGINT NOT NULL DEFAULT 1,
    PRIMARY KEY (organization_id)
);

CREATE TABLE purchase_order_lines (
    id          UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    po_id       UUID    NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id  UUID    REFERENCES products(id) ON DELETE SET NULL,
    description TEXT    NOT NULL,
    quantity    BIGINT  NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price  BIGINT  NOT NULL DEFAULT 0,
    tax_rate    BIGINT  NOT NULL DEFAULT 0,
    quantity_received BIGINT NOT NULL DEFAULT 0,
    sort_order  INT     NOT NULL DEFAULT 0
);

CREATE INDEX idx_po_org    ON purchase_orders (organization_id);
CREATE INDEX idx_po_status ON purchase_orders (organization_id, status);
CREATE INDEX idx_po_lines  ON purchase_order_lines (po_id);
