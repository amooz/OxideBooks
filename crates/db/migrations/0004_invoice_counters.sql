-- Atomic per-org invoice number counter.
-- Replaces the COUNT(*)+1 approach which had a race condition under concurrent inserts.
CREATE TABLE invoice_counters (
    organization_id UUID    NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    invoice_type    TEXT    NOT NULL,  -- 'invoice' | 'bill'
    next_val        BIGINT  NOT NULL DEFAULT 1,
    PRIMARY KEY (organization_id, invoice_type)
);
