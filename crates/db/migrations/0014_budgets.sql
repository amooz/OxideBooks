CREATE TABLE budgets (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    fiscal_year     INT         NOT NULL,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_budgets_org ON budgets (organization_id);

CREATE TABLE budget_lines (
    id         UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    budget_id  UUID    NOT NULL REFERENCES budgets(id) ON DELETE CASCADE,
    account_id UUID    NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    month      INT     NOT NULL CHECK (month BETWEEN 1 AND 12),
    amount     BIGINT  NOT NULL DEFAULT 0,
    UNIQUE (budget_id, account_id, month)
);

CREATE INDEX idx_budget_lines_budget ON budget_lines (budget_id);
