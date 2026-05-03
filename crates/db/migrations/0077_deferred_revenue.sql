-- Deferred revenue: recognize invoice revenue over time
CREATE TABLE deferred_revenue_schedules (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id          UUID REFERENCES invoices(id) ON DELETE SET NULL,
    invoice_line_id     UUID,
    deferred_account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    revenue_account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    description         TEXT NOT NULL,
    total_amount        BIGINT NOT NULL CHECK (total_amount > 0),
    recognized_amount   BIGINT NOT NULL DEFAULT 0,
    start_date          DATE NOT NULL,
    end_date            DATE NOT NULL,
    frequency           TEXT NOT NULL DEFAULT 'monthly'
                        CHECK (frequency IN ('daily','weekly','monthly','quarterly','annually')),
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active','completed','cancelled')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_date > start_date)
);

CREATE TABLE deferred_revenue_entries (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id         UUID NOT NULL REFERENCES deferred_revenue_schedules(id) ON DELETE CASCADE,
    recognition_date    DATE NOT NULL,
    amount              BIGINT NOT NULL CHECK (amount > 0),
    journal_entry_id    UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dr_sched_org      ON deferred_revenue_schedules(organization_id);
CREATE INDEX idx_dr_sched_invoice  ON deferred_revenue_schedules(invoice_id) WHERE invoice_id IS NOT NULL;
CREATE INDEX idx_dr_entries_sched  ON deferred_revenue_entries(schedule_id);
