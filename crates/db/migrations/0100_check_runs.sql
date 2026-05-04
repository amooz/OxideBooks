CREATE TABLE check_runs (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    bank_account_id      UUID        NOT NULL REFERENCES bank_accounts(id),
    run_date             DATE        NOT NULL,
    status               TEXT        NOT NULL DEFAULT 'draft'
                             CHECK (status IN ('draft', 'printed', 'voided')),
    starting_check_number INT,
    notes                TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE check_run_items (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    check_run_id  UUID        NOT NULL REFERENCES check_runs(id) ON DELETE CASCADE,
    payee_id      UUID        REFERENCES contacts(id),
    payee_name    TEXT        NOT NULL,
    amount        BIGINT      NOT NULL CHECK (amount > 0),
    memo          TEXT,
    check_number  INT,
    status        TEXT        NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending', 'printed', 'voided')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
