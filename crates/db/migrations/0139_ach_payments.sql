CREATE TYPE ach_entry_type AS ENUM ('collection', 'payment');
CREATE TYPE ach_payment_status AS ENUM ('pending', 'submitted', 'settled', 'returned');

CREATE TABLE ach_payments (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    entry_type       ach_entry_type NOT NULL,
    invoice_id       UUID REFERENCES invoices(id),
    bill_id          UUID REFERENCES vendor_bills(id),
    routing_number   TEXT NOT NULL,
    account_number   TEXT NOT NULL,
    account_type     TEXT NOT NULL DEFAULT 'checking'
                         CHECK (account_type IN ('checking', 'savings')),
    amount           BIGINT NOT NULL CHECK (amount > 0),
    status           ach_payment_status NOT NULL DEFAULT 'pending',
    trace_number     TEXT,
    effective_date   DATE NOT NULL,
    return_code      TEXT,
    submitted_at     TIMESTAMPTZ,
    settled_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT ach_payments_ref_check CHECK (
        (invoice_id IS NOT NULL AND bill_id IS NULL) OR
        (bill_id IS NOT NULL AND invoice_id IS NULL)
    )
);

CREATE INDEX ach_payments_org_idx ON ach_payments(organization_id);
CREATE INDEX ach_payments_invoice_idx ON ach_payments(invoice_id);
CREATE INDEX ach_payments_bill_idx ON ach_payments(bill_id);
