CREATE TABLE batch_payments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    payment_date    DATE NOT NULL,
    method          TEXT NOT NULL,
    reference       TEXT,
    total_amount    BIGINT NOT NULL DEFAULT 0,
    created_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE batch_payment_lines (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_payment_id UUID NOT NULL REFERENCES batch_payments(id) ON DELETE CASCADE,
    invoice_id       UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount           BIGINT NOT NULL
);

CREATE INDEX batch_payments_org ON batch_payments (organization_id, created_at DESC);
CREATE INDEX batch_payment_lines_batch ON batch_payment_lines (batch_payment_id);
