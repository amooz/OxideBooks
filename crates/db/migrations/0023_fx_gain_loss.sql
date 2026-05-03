CREATE TABLE realized_fx_entries (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    payment_id      UUID        NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    invoice_currency TEXT       NOT NULL,
    payment_currency TEXT       NOT NULL,
    invoice_amount  BIGINT      NOT NULL,
    payment_amount  BIGINT      NOT NULL,
    fx_rate         FLOAT8      NOT NULL,
    gain_loss       BIGINT      NOT NULL,
    journal_entry_id UUID       REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (payment_id)
);

CREATE INDEX idx_fx_entries_org ON realized_fx_entries (organization_id, created_at DESC);
