-- Bank deposits: group multiple payments into a single deposit to the bank account.
CREATE TABLE bank_deposits (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    bank_account_id UUID NOT NULL REFERENCES bank_accounts(id) ON DELETE RESTRICT,
    deposit_date    DATE NOT NULL,
    currency        TEXT NOT NULL DEFAULT 'USD',
    total_amount    BIGINT NOT NULL DEFAULT 0,
    reference       TEXT,
    memo            TEXT,
    status          TEXT NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'cleared')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bank_deposit_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deposit_id  UUID NOT NULL REFERENCES bank_deposits(id) ON DELETE CASCADE,
    payment_id  UUID NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    amount      BIGINT NOT NULL,
    UNIQUE (deposit_id, payment_id)
);

CREATE INDEX idx_dep_org  ON bank_deposits(organization_id);
CREATE INDEX idx_dep_date ON bank_deposits(organization_id, deposit_date DESC);
