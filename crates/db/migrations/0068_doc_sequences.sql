-- Per-organization document number sequences (INV-0001, BILL-0042, etc.)
CREATE TABLE doc_sequences (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    doc_type        TEXT NOT NULL CHECK (doc_type IN (
                        'invoice', 'bill', 'credit_note', 'purchase_order',
                        'quote', 'expense_report', 'payment')),
    prefix          TEXT NOT NULL DEFAULT '',
    next_number     BIGINT NOT NULL DEFAULT 1 CHECK (next_number >= 1),
    pad_length      INTEGER NOT NULL DEFAULT 4 CHECK (pad_length BETWEEN 1 AND 10),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, doc_type)
);
