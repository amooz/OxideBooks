CREATE TABLE vendor_bills (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id       UUID        REFERENCES contacts(id) ON DELETE SET NULL,
    bill_date        DATE        NOT NULL,
    due_date         DATE,
    reference        TEXT,
    description      TEXT        NOT NULL DEFAULT '',
    status           TEXT        NOT NULL DEFAULT 'draft'
                                 CHECK (status IN ('draft','approved','partial','paid','voided')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bill_lines (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    bill_id          UUID        NOT NULL REFERENCES vendor_bills(id) ON DELETE CASCADE,
    account_id       UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    description      TEXT,
    quantity         INT         NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price       BIGINT      NOT NULL CHECK (unit_price >= 0),
    tax_rate         BIGINT      NOT NULL DEFAULT 0 CHECK (tax_rate >= 0)
);

CREATE TABLE bill_payments (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    bill_id          UUID        NOT NULL REFERENCES vendor_bills(id) ON DELETE CASCADE,
    payment_date     DATE        NOT NULL,
    amount           BIGINT      NOT NULL CHECK (amount > 0),
    method           TEXT        NOT NULL DEFAULT 'bank_transfer',
    reference        TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX vendor_bills_org      ON vendor_bills (organization_id);
CREATE INDEX vendor_bills_contact  ON vendor_bills (contact_id) WHERE contact_id IS NOT NULL;
CREATE INDEX bill_payments_bill    ON bill_payments (bill_id);
