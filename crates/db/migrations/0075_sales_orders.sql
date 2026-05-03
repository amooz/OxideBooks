-- Sales orders: customer commitment before invoicing
CREATE TABLE sales_orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    order_number    TEXT NOT NULL,
    contact_id      UUID NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    status          TEXT NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft','confirmed','partially_invoiced',
                                      'fully_invoiced','cancelled')),
    order_date      DATE NOT NULL,
    expected_ship   DATE,
    currency        TEXT NOT NULL DEFAULT 'USD',
    notes           TEXT,
    total_amount    BIGINT NOT NULL DEFAULT 0,
    invoiced_amount BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, order_number)
);

CREATE TABLE sales_order_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    so_id           UUID NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id) ON DELETE SET NULL,
    description     TEXT NOT NULL,
    quantity        BIGINT NOT NULL DEFAULT 100 CHECK (quantity > 0),
    unit_price      BIGINT NOT NULL CHECK (unit_price >= 0),
    tax_rate        BIGINT NOT NULL DEFAULT 0,
    discount_pct    BIGINT NOT NULL DEFAULT 0,
    quantity_invoiced BIGINT NOT NULL DEFAULT 0,
    sort_order      INT NOT NULL DEFAULT 0
);

-- Counter for SO numbering
CREATE TABLE so_counters (
    organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    next_val        BIGINT NOT NULL DEFAULT 1
);

-- Track which SOs an invoice was created from
ALTER TABLE invoices ADD COLUMN sales_order_id UUID REFERENCES sales_orders(id) ON DELETE SET NULL;

CREATE INDEX idx_so_org     ON sales_orders(organization_id);
CREATE INDEX idx_so_contact ON sales_orders(contact_id);
CREATE INDEX idx_so_lines   ON sales_order_lines(so_id);
CREATE INDEX idx_invoices_so ON invoices(sales_order_id) WHERE sales_order_id IS NOT NULL;
