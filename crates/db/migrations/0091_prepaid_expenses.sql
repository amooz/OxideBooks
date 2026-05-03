-- Prepaid expense amortization schedules (e.g. insurance, rent paid in advance)
CREATE TABLE prepaid_expense_schedules (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    description         TEXT        NOT NULL,
    total_amount        BIGINT      NOT NULL CHECK (total_amount > 0),
    -- account that holds the prepaid asset (Balance Sheet)
    asset_account_id    UUID        NOT NULL REFERENCES accounts(id),
    -- account to recognize expense into each period (P&L)
    expense_account_id  UUID        NOT NULL REFERENCES accounts(id),
    start_date          DATE        NOT NULL,
    end_date            DATE        NOT NULL,
    -- 'monthly' or 'custom'
    frequency           TEXT        NOT NULL DEFAULT 'monthly'
                            CHECK (frequency IN ('monthly', 'custom')),
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    amortized_amount    BIGINT      NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_date > start_date)
);

CREATE TABLE prepaid_expense_entries (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id         UUID        NOT NULL REFERENCES prepaid_expense_schedules(id) ON DELETE CASCADE,
    period_date         DATE        NOT NULL,
    amount              BIGINT      NOT NULL CHECK (amount > 0),
    journal_entry_id    UUID        REFERENCES journal_entries(id) ON DELETE SET NULL,
    recognized_at       TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON prepaid_expense_schedules (organization_id);
CREATE INDEX ON prepaid_expense_entries (schedule_id);
CREATE INDEX ON prepaid_expense_entries (period_date);
