CREATE TABLE landed_costs (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    grn_id              UUID        NOT NULL REFERENCES goods_receipt_notes(id) ON DELETE CASCADE,
    description         TEXT        NOT NULL,
    amount              BIGINT      NOT NULL CHECK (amount > 0),
    -- 'quantity' splits evenly by units received; 'value' splits by line value
    allocation_method   TEXT        NOT NULL DEFAULT 'quantity'
                            CHECK (allocation_method IN ('quantity', 'value')),
    currency            TEXT        NOT NULL DEFAULT 'USD',
    vendor_id           UUID        REFERENCES contacts(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE landed_cost_allocations (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    landed_cost_id      UUID        NOT NULL REFERENCES landed_costs(id) ON DELETE CASCADE,
    grn_line_id         UUID        NOT NULL REFERENCES grn_lines(id) ON DELETE CASCADE,
    allocated_amount    BIGINT      NOT NULL
);

CREATE INDEX ON landed_costs (organization_id);
CREATE INDEX ON landed_costs (grn_id);
