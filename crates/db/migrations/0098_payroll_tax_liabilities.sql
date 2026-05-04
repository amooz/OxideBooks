-- Payroll tax liabilities: accumulate employer + employee tax obligations per payroll run.
CREATE TABLE payroll_tax_liabilities (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    payroll_run_id   UUID        NOT NULL REFERENCES payroll_runs(id) ON DELETE CASCADE,
    tax_type         TEXT        NOT NULL,  -- e.g. 'federal_income', 'state_income',
                                            --       'social_security', 'medicare', 'futa', 'suta'
    employee_amount  BIGINT      NOT NULL DEFAULT 0 CHECK (employee_amount >= 0),
    employer_amount  BIGINT      NOT NULL DEFAULT 0 CHECK (employer_amount >= 0),
    period_start     DATE        NOT NULL,
    period_end       DATE        NOT NULL,
    due_date         DATE,
    paid_date        DATE,
    status           TEXT        NOT NULL DEFAULT 'accrued'
                                 CHECK (status IN ('accrued', 'paid', 'voided')),
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ptl_org ON payroll_tax_liabilities(organization_id);
CREATE INDEX idx_ptl_run ON payroll_tax_liabilities(payroll_run_id);
CREATE UNIQUE INDEX idx_ptl_run_type ON payroll_tax_liabilities(payroll_run_id, tax_type);
