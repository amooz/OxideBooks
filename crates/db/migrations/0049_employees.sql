CREATE TABLE employees (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    first_name      TEXT NOT NULL,
    last_name       TEXT NOT NULL,
    email           TEXT,
    employee_number TEXT,
    start_date      DATE NOT NULL,
    terminated_at   DATE,
    pay_type        TEXT NOT NULL CHECK (pay_type IN ('salary', 'hourly')),
    pay_rate        BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX employees_org ON employees (organization_id);
CREATE UNIQUE INDEX employees_org_number ON employees (organization_id, employee_number)
    WHERE employee_number IS NOT NULL;
