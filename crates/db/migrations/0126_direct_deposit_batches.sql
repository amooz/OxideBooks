-- Payroll direct deposit batch — aggregates ACH entries for a payroll run
CREATE TABLE direct_deposit_batches (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    payroll_run_id      UUID        REFERENCES payroll_runs(id),
    bank_account_id     UUID        REFERENCES bank_accounts(id),
    batch_date          DATE        NOT NULL,
    status              TEXT        NOT NULL DEFAULT 'pending'
                            CHECK (status IN ('pending','sent','cleared','failed')),
    total_amount        BIGINT      NOT NULL DEFAULT 0,
    entry_count         INT         NOT NULL DEFAULT 0,
    reference           TEXT,
    sent_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE direct_deposit_entries (
    id                  UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id            UUID    NOT NULL REFERENCES direct_deposit_batches(id) ON DELETE CASCADE,
    employee_id         UUID    NOT NULL REFERENCES employees(id),
    employee_bank_id    UUID    REFERENCES employee_bank_accounts(id),
    amount              BIGINT  NOT NULL CHECK (amount > 0),
    routing_number      TEXT,
    account_number      TEXT,
    account_type        TEXT    NOT NULL DEFAULT 'checking'
                            CHECK (account_type IN ('checking','savings'))
);

CREATE INDEX idx_dd_batches_org        ON direct_deposit_batches(organization_id);
CREATE INDEX idx_dd_batches_run        ON direct_deposit_batches(payroll_run_id);
CREATE INDEX idx_dd_entries_batch      ON direct_deposit_entries(batch_id);
CREATE INDEX idx_dd_entries_employee   ON direct_deposit_entries(employee_id);
