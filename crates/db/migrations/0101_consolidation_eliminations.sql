CREATE TABLE consolidation_eliminations (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    intercompany_link_id UUID        REFERENCES intercompany_links(id),
    period_start         DATE        NOT NULL,
    period_end           DATE        NOT NULL,
    debit_account_id     UUID        NOT NULL REFERENCES accounts(id),
    credit_account_id    UUID        NOT NULL REFERENCES accounts(id),
    amount               BIGINT      NOT NULL CHECK (amount > 0),
    description          TEXT        NOT NULL DEFAULT '',
    status               TEXT        NOT NULL DEFAULT 'active'
                             CHECK (status IN ('active', 'voided')),
    notes                TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (debit_account_id != credit_account_id)
);

CREATE INDEX idx_cons_elim_org_period
    ON consolidation_eliminations(organization_id, period_end DESC);
