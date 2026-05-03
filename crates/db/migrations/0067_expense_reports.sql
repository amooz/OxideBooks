CREATE TABLE expense_reports (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    title           TEXT        NOT NULL,
    employee_id     UUID        REFERENCES employees(id) ON DELETE SET NULL,
    notes           TEXT,
    status          TEXT        NOT NULL DEFAULT 'draft'
                                CHECK (status IN ('draft', 'submitted', 'approved', 'reimbursed', 'rejected')),
    total_amount    BIGINT      NOT NULL DEFAULT 0,
    approved_by     UUID        REFERENCES users(id),
    approved_at     TIMESTAMPTZ,
    reimbursed_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_expense_reports_org      ON expense_reports(organization_id);
CREATE INDEX idx_expense_reports_employee ON expense_reports(organization_id, employee_id);

ALTER TABLE expenses ADD COLUMN expense_report_id UUID REFERENCES expense_reports(id) ON DELETE SET NULL;
