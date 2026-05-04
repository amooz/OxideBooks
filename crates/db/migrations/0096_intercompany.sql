-- Intercompany accounting: track relationships between orgs and link paired JEs.

-- Defines that org_a and org_b are related entities; stores the GL accounts
-- used for the intercompany payable/receivable on each side.
CREATE TABLE intercompany_links (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    counterparty_org_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    due_from_account_id  UUID        REFERENCES accounts(id),  -- AR / Due From on this side
    due_to_account_id    UUID        REFERENCES accounts(id),  -- AP / Due To on this side
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, counterparty_org_id)
);

-- Each intercompany transaction links two journal entries (one per org).
CREATE TABLE intercompany_transactions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_a_id        UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    journal_entry_a UUID        NOT NULL REFERENCES journal_entries(id),
    org_b_id        UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    journal_entry_b UUID        NOT NULL REFERENCES journal_entries(id),
    amount          BIGINT      NOT NULL,
    currency        TEXT        NOT NULL DEFAULT 'USD',
    description     TEXT,
    transaction_date DATE       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_interco_org_a ON intercompany_transactions(org_a_id);
CREATE INDEX idx_interco_org_b ON intercompany_transactions(org_b_id);
