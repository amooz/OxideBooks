CREATE TABLE bank_accounts (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    account_number  TEXT,
    institution     TEXT,
    currency        TEXT        NOT NULL DEFAULT 'USD',
    current_balance BIGINT      NOT NULL DEFAULT 0,
    gl_account_id   UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE bank_transactions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_account_id UUID        NOT NULL REFERENCES bank_accounts(id) ON DELETE CASCADE,
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    txn_date        DATE        NOT NULL,
    description     TEXT        NOT NULL,
    amount          BIGINT      NOT NULL,
    txn_type        TEXT        NOT NULL CHECK (txn_type IN ('debit','credit')),
    status          TEXT        NOT NULL DEFAULT 'unmatched'
                    CHECK (status IN ('unmatched','matched','excluded')),
    reference       TEXT,
    matched_payment_id UUID     REFERENCES payments(id) ON DELETE SET NULL,
    matched_expense_id UUID     REFERENCES expenses(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bank_accounts_org  ON bank_accounts (organization_id);
CREATE INDEX idx_bank_txns_account  ON bank_transactions (bank_account_id, txn_date DESC);
CREATE INDEX idx_bank_txns_status   ON bank_transactions (organization_id, status);
