CREATE TABLE assembly_orders (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    product_id      UUID        NOT NULL REFERENCES products(id),
    quantity        INT         NOT NULL CHECK (quantity > 0),
    status          TEXT        NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'built', 'cancelled')),
    build_date      DATE,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE assembly_order_lines (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    assembly_order_id UUID        NOT NULL REFERENCES assembly_orders(id) ON DELETE CASCADE,
    component_id      UUID        NOT NULL REFERENCES products(id),
    quantity_required INT         NOT NULL CHECK (quantity_required > 0),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
