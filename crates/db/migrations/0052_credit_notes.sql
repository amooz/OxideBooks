CREATE TABLE credit_notes (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id       UUID        REFERENCES contacts(id) ON DELETE SET NULL,
    note_date        DATE        NOT NULL,
    reference        TEXT,
    description      TEXT        NOT NULL DEFAULT '',
    amount           BIGINT      NOT NULL CHECK (amount > 0),
    remaining        BIGINT      NOT NULL,
    status           TEXT        NOT NULL DEFAULT 'open'
                                 CHECK (status IN ('open','partial','applied','voided')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE credit_note_applications (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    credit_note_id   UUID        NOT NULL REFERENCES credit_notes(id) ON DELETE CASCADE,
    invoice_id       UUID        NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount_applied   BIGINT      NOT NULL CHECK (amount_applied > 0),
    applied_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX credit_notes_org      ON credit_notes (organization_id);
CREATE INDEX credit_notes_contact  ON credit_notes (contact_id) WHERE contact_id IS NOT NULL;
CREATE INDEX cna_credit_note       ON credit_note_applications (credit_note_id);
CREATE INDEX cna_invoice           ON credit_note_applications (invoice_id);
