CREATE TYPE rev_rec_method AS ENUM ('straight_line', 'milestone', 'usage_based', 'manual');
CREATE TYPE rev_rec_status AS ENUM ('draft', 'active', 'completed', 'cancelled');

CREATE TABLE rev_rec_schedules (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id          UUID REFERENCES invoices(id),
    revenue_account_id  UUID REFERENCES accounts(id),
    deferred_account_id UUID REFERENCES accounts(id),
    description         TEXT NOT NULL,
    method              rev_rec_method NOT NULL DEFAULT 'straight_line',
    total_amount        BIGINT NOT NULL CHECK (total_amount > 0),
    recognized_amount   BIGINT NOT NULL DEFAULT 0,
    start_date          DATE NOT NULL,
    end_date            DATE NOT NULL,
    status              rev_rec_status NOT NULL DEFAULT 'active',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (end_date >= start_date)
);

CREATE TABLE rev_rec_entries (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id      UUID NOT NULL REFERENCES rev_rec_schedules(id) ON DELETE CASCADE,
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period           DATE NOT NULL,
    amount           BIGINT NOT NULL CHECK (amount > 0),
    journal_entry_id UUID REFERENCES journal_entries(id),
    posted_at        TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (schedule_id, period)
);

CREATE INDEX rev_rec_schedules_org_idx ON rev_rec_schedules(organization_id);
CREATE INDEX rev_rec_schedules_invoice_idx ON rev_rec_schedules(invoice_id);
CREATE INDEX rev_rec_entries_schedule_idx ON rev_rec_entries(schedule_id);
CREATE INDEX rev_rec_entries_org_period_idx ON rev_rec_entries(organization_id, period);
