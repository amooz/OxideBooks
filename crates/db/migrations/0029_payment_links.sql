CREATE TABLE payment_links (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id      UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    token           TEXT NOT NULL UNIQUE,
    amount_due      BIGINT NOT NULL,
    currency        TEXT NOT NULL DEFAULT 'USD',
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paid', 'expired', 'cancelled')),
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payment_links_invoice ON payment_links(invoice_id);
CREATE INDEX idx_payment_links_token   ON payment_links(token);
