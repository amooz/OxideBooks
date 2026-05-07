CREATE TABLE quotes (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id   UUID NOT NULL REFERENCES organizations(id),
    contact_id        UUID REFERENCES contacts(id),
    quote_number      TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft','sent','accepted','declined','expired','invoiced')),
    issue_date        DATE NOT NULL,
    expiry_date       DATE,
    currency          TEXT NOT NULL DEFAULT 'USD',
    exchange_rate     NUMERIC(20,6) NOT NULL DEFAULT 1,
    notes             TEXT,
    terms             TEXT,
    sent_at           TIMESTAMPTZ,
    accepted_at       TIMESTAMPTZ,
    declined_at       TIMESTAMPTZ,
    invoiced_at       TIMESTAMPTZ,
    converted_invoice_id UUID REFERENCES invoices(id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX quotes_org_number ON quotes(organization_id, quote_number);
CREATE INDEX quotes_org_contact ON quotes(organization_id, contact_id);
CREATE INDEX quotes_org_status ON quotes(organization_id, status);

CREATE TABLE quote_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    quote_id        UUID NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id),
    description     TEXT NOT NULL,
    quantity        BIGINT NOT NULL DEFAULT 100,
    unit_price      BIGINT NOT NULL DEFAULT 0,
    discount_pct    BIGINT NOT NULL DEFAULT 0,
    tax_rate        BIGINT NOT NULL DEFAULT 0,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX quote_lines_quote ON quote_lines(quote_id);
