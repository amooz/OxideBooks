-- Recurring journal entries (memorized transactions in QB parlance)
-- Extends the existing recurring_schedules table with a JE template payload.
-- The payload is stored as JSONB so the existing recurring infra can drive it.
-- We also add a dedicated memo-transaction table for truly standalone recurring JEs.
CREATE TABLE recurring_journal_entries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    frequency       TEXT NOT NULL DEFAULT 'monthly'
                        CHECK (frequency IN ('daily','weekly','biweekly','monthly','quarterly','yearly')),
    next_date       DATE NOT NULL,
    end_date        DATE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    auto_post       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE recurring_journal_entry_lines (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recurring_journal_entry_id  UUID NOT NULL
                                    REFERENCES recurring_journal_entries(id) ON DELETE CASCADE,
    account_id  UUID NOT NULL REFERENCES accounts(id),
    description TEXT,
    debit       BIGINT NOT NULL DEFAULT 0 CHECK (debit >= 0),
    credit      BIGINT NOT NULL DEFAULT 0 CHECK (credit >= 0),
    CHECK (debit > 0 OR credit > 0),
    CHECK (NOT (debit > 0 AND credit > 0))
);

CREATE INDEX idx_rec_je_org ON recurring_journal_entries(organization_id, next_date);
