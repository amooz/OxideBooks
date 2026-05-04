-- Sales commissions: track commissions owed to salespeople on invoices.
CREATE TABLE sales_commissions (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id       UUID        NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    salesperson_id   UUID        NOT NULL REFERENCES contacts(id),
    rate_bps         INT         NOT NULL CHECK (rate_bps >= 0 AND rate_bps <= 100000), -- basis points (1% = 100)
    amount           BIGINT      NOT NULL CHECK (amount >= 0),                           -- minor units
    status           TEXT        NOT NULL DEFAULT 'pending'
                                 CHECK (status IN ('pending','approved','paid','voided')),
    payment_date     DATE,
    payment_ref      TEXT,
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sales_commissions_org    ON sales_commissions(organization_id);
CREATE INDEX idx_sales_commissions_inv    ON sales_commissions(invoice_id);
CREATE INDEX idx_sales_commissions_person ON sales_commissions(salesperson_id);
