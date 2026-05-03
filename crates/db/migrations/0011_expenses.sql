CREATE TABLE expenses (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    expense_date    DATE        NOT NULL,
    amount          BIGINT      NOT NULL CHECK (amount > 0),
    currency        TEXT        NOT NULL DEFAULT 'USD',
    category        TEXT        NOT NULL,
    description     TEXT        NOT NULL,
    account_id      UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    status          TEXT        NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft','submitted','approved','rejected','reimbursed')),
    receipt_url     TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_expenses_org      ON expenses (organization_id, created_at DESC);
CREATE INDEX idx_expenses_user     ON expenses (organization_id, user_id);
CREATE INDEX idx_expenses_status   ON expenses (organization_id, status);
