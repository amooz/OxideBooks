-- Deferred charges: billable charges recorded now, invoiced later (QB delayed charges)
CREATE TABLE deferred_charges (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id      UUID NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    account_id      UUID REFERENCES accounts(id) ON DELETE SET NULL,
    description     TEXT NOT NULL,
    charge_date     DATE NOT NULL,
    amount          BIGINT NOT NULL CHECK (amount > 0),
    tax_rate        BIGINT NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','invoiced','void')),
    invoice_id      UUID REFERENCES invoices(id) ON DELETE SET NULL,
    invoiced_at     TIMESTAMPTZ,
    memo            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dc_org     ON deferred_charges(organization_id);
CREATE INDEX idx_dc_contact ON deferred_charges(contact_id);
CREATE INDEX idx_dc_status  ON deferred_charges(organization_id, status);
