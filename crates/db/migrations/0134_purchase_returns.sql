CREATE TABLE purchase_returns (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id),
    bill_id             UUID REFERENCES vendor_bills(id),
    contact_id          UUID REFERENCES contacts(id),
    rma_number          TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'requested'
                            CHECK (status IN ('requested','approved','shipped','closed')),
    reason              TEXT,
    notes               TEXT,
    vendor_credit_id    UUID REFERENCES vendor_credits(id),
    approved_at         TIMESTAMPTZ,
    shipped_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX purchase_returns_org_rma ON purchase_returns(organization_id, rma_number);
CREATE INDEX purchase_returns_org_status ON purchase_returns(organization_id, status);
CREATE INDEX purchase_returns_org_bill ON purchase_returns(organization_id, bill_id);

CREATE TABLE purchase_return_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    return_id       UUID NOT NULL REFERENCES purchase_returns(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id),
    description     TEXT NOT NULL,
    quantity        BIGINT NOT NULL DEFAULT 100,
    unit_price      BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX purchase_return_lines_return ON purchase_return_lines(return_id);
