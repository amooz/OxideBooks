CREATE TABLE payment_terms (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    net_days        INTEGER     NOT NULL DEFAULT 30 CHECK (net_days >= 0),
    -- Early-payment discount: number of days within which the discount applies
    discount_days   INTEGER,
    -- Discount percent × 100 (e.g. 2% → 200); 0 = no discount
    discount_pct    BIGINT      NOT NULL DEFAULT 0 CHECK (discount_pct >= 0),
    is_default      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

ALTER TABLE contacts ADD COLUMN payment_terms_id UUID REFERENCES payment_terms(id) ON DELETE SET NULL;
ALTER TABLE invoices  ADD COLUMN payment_terms_id UUID REFERENCES payment_terms(id) ON DELETE SET NULL;
