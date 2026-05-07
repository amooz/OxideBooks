CREATE TABLE bank_feed_transactions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id),
    bank_account_id     UUID NOT NULL REFERENCES bank_accounts(id),
    txn_date            DATE NOT NULL,
    description         TEXT NOT NULL,
    amount              BIGINT NOT NULL,
    txn_type            TEXT NOT NULL DEFAULT 'debit' CHECK (txn_type IN ('debit','credit')),
    reference           TEXT,
    source              TEXT NOT NULL DEFAULT 'csv' CHECK (source IN ('csv','ofx','qfx','manual')),
    status              TEXT NOT NULL DEFAULT 'pending'
                            CHECK (status IN ('pending','matched','ignored')),
    matched_txn_id      UUID REFERENCES bank_transactions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX bank_feed_org_account ON bank_feed_transactions(organization_id, bank_account_id);
CREATE INDEX bank_feed_status ON bank_feed_transactions(organization_id, status);
CREATE UNIQUE INDEX bank_feed_dedup ON bank_feed_transactions(bank_account_id, txn_date, amount, description);
