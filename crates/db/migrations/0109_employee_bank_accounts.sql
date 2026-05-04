-- Employee bank accounts for direct deposit payroll
CREATE TABLE employee_bank_accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    employee_id     UUID NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    bank_name       TEXT NOT NULL,
    routing_number  TEXT NOT NULL,
    account_last4   TEXT NOT NULL CHECK (account_last4 ~ '^[0-9]{4}$'),
    account_type    TEXT NOT NULL DEFAULT 'checking'
                        CHECK (account_type IN ('checking', 'savings')),
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_emp_bank_accts_employee ON employee_bank_accounts(employee_id);

-- Add direct_deposit_enabled flag to employees
ALTER TABLE employees ADD COLUMN direct_deposit_enabled BOOLEAN NOT NULL DEFAULT FALSE;
