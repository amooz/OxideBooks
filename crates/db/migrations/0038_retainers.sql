CREATE TABLE retainers (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id      UUID        NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    currency        TEXT        NOT NULL DEFAULT 'USD',
    balance_cents   BIGINT      NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON retainers(organization_id);
CREATE INDEX ON retainers(contact_id);

CREATE TABLE retainer_transactions (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    retainer_id  UUID        NOT NULL REFERENCES retainers(id) ON DELETE CASCADE,
    invoice_id   UUID        REFERENCES invoices(id) ON DELETE SET NULL,
    amount       BIGINT      NOT NULL,
    txn_type     TEXT        NOT NULL, -- 'deposit' | 'applied'
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON retainer_transactions(retainer_id);
