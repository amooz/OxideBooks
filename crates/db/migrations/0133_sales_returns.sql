CREATE TABLE sales_returns (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id),
    invoice_id          UUID REFERENCES invoices(id),
    contact_id          UUID REFERENCES contacts(id),
    rma_number          TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'requested'
                            CHECK (status IN ('requested','approved','received','closed')),
    reason              TEXT,
    notes               TEXT,
    credit_note_id      UUID REFERENCES credit_notes(id),
    approved_at         TIMESTAMPTZ,
    received_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX sales_returns_org_rma ON sales_returns(organization_id, rma_number);
CREATE INDEX sales_returns_org_status ON sales_returns(organization_id, status);
CREATE INDEX sales_returns_org_invoice ON sales_returns(organization_id, invoice_id);

CREATE TABLE sales_return_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    return_id       UUID NOT NULL REFERENCES sales_returns(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id),
    description     TEXT NOT NULL,
    quantity        BIGINT NOT NULL DEFAULT 100,
    unit_price      BIGINT NOT NULL DEFAULT 0,
    restock         BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX sales_return_lines_return ON sales_return_lines(return_id);
