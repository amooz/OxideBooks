CREATE TABLE payments (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id      UUID        NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount          BIGINT      NOT NULL CHECK (amount > 0),
    payment_date    DATE        NOT NULL,
    method          TEXT        NOT NULL DEFAULT 'bank_transfer',
    reference       TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payments_invoice ON payments (organization_id, invoice_id);
