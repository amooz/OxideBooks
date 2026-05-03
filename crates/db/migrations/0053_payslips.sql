CREATE TABLE payslips (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    payroll_run_id   UUID        NOT NULL REFERENCES payroll_runs(id) ON DELETE CASCADE,
    employee_id      UUID        NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    gross_pay        BIGINT      NOT NULL CHECK (gross_pay >= 0),
    tax_withheld     BIGINT      NOT NULL DEFAULT 0 CHECK (tax_withheld >= 0),
    deductions       BIGINT      NOT NULL DEFAULT 0 CHECK (deductions >= 0),
    net_pay          BIGINT      NOT NULL CHECK (net_pay >= 0),
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (payroll_run_id, employee_id)
);

CREATE INDEX payslips_org         ON payslips (organization_id);
CREATE INDEX payslips_run         ON payslips (payroll_run_id);
CREATE INDEX payslips_employee    ON payslips (employee_id);
