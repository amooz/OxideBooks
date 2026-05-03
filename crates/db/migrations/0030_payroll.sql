CREATE TABLE payroll_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'approved', 'paid')),
    journal_entry_id UUID REFERENCES journal_entries(id),
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE payroll_entries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payroll_run_id  UUID NOT NULL REFERENCES payroll_runs(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id),
    gross_pay       BIGINT NOT NULL CHECK (gross_pay >= 0),
    tax_withheld    BIGINT NOT NULL DEFAULT 0 CHECK (tax_withheld >= 0),
    other_deductions BIGINT NOT NULL DEFAULT 0 CHECK (other_deductions >= 0),
    net_pay         BIGINT GENERATED ALWAYS AS (gross_pay - tax_withheld - other_deductions) STORED,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payroll_runs_org ON payroll_runs(organization_id, period_start DESC);
CREATE INDEX idx_payroll_entries_run ON payroll_entries(payroll_run_id);
