CREATE TABLE bank_reconciliation_statements (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id),
    bank_account_id     UUID NOT NULL REFERENCES bank_accounts(id),
    statement_date      DATE NOT NULL,
    statement_balance   BIGINT NOT NULL,
    book_balance        BIGINT NOT NULL,
    outstanding_deposits BIGINT NOT NULL DEFAULT 0,
    outstanding_checks  BIGINT NOT NULL DEFAULT 0,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, bank_account_id, statement_date)
);

CREATE INDEX ON bank_reconciliation_statements (organization_id, bank_account_id);
