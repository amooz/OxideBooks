-- Employee loans and advances: track money lent to employees, repaid via payroll deductions.
CREATE TABLE employee_loans (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    employee_id      UUID        NOT NULL REFERENCES employees(id),
    amount           BIGINT      NOT NULL CHECK (amount > 0),
    balance          BIGINT      NOT NULL CHECK (balance >= 0),  -- remaining unpaid amount
    purpose          TEXT,
    account_id       UUID        REFERENCES accounts(id),        -- GL account for the advance
    loan_date        DATE        NOT NULL,
    status           TEXT        NOT NULL DEFAULT 'active'
                                 CHECK (status IN ('active', 'paid_off', 'written_off')),
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_employee_loans_org      ON employee_loans(organization_id);
CREATE INDEX idx_employee_loans_employee ON employee_loans(employee_id);

CREATE TABLE employee_loan_repayments (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    loan_id          UUID        NOT NULL REFERENCES employee_loans(id) ON DELETE CASCADE,
    repayment_date   DATE        NOT NULL,
    amount           BIGINT      NOT NULL CHECK (amount > 0),
    payslip_id       UUID        REFERENCES payslips(id),
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_loan_repayments_loan ON employee_loan_repayments(loan_id);
