CREATE TABLE contractor_tax_info (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id       UUID        NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    tax_id_type      TEXT        NOT NULL DEFAULT 'ein'
                         CHECK (tax_id_type IN ('ein', 'ssn', 'itin')),
    tax_id_last4     TEXT        NOT NULL,
    business_type    TEXT        NOT NULL DEFAULT 'individual'
                         CHECK (business_type IN (
                             'individual', 'sole_proprietor', 'partnership',
                             'llc', 'corporation', 'other'
                         )),
    form_1099_type   TEXT        NOT NULL DEFAULT 'NEC'
                         CHECK (form_1099_type IN ('NEC', 'MISC')),
    w9_received_date DATE,
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, contact_id)
);
