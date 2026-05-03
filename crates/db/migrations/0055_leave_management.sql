CREATE TABLE leave_types (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT        NOT NULL,
    days_per_year    NUMERIC(5,2) NOT NULL DEFAULT 0 CHECK (days_per_year >= 0),
    is_paid          BOOL        NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE leave_requests (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    employee_id      UUID        NOT NULL REFERENCES employees(id) ON DELETE CASCADE,
    leave_type_id    UUID        NOT NULL REFERENCES leave_types(id) ON DELETE RESTRICT,
    start_date       DATE        NOT NULL,
    end_date         DATE        NOT NULL,
    days             NUMERIC(5,2) NOT NULL CHECK (days > 0),
    status           TEXT        NOT NULL DEFAULT 'pending'
                                 CHECK (status IN ('pending','approved','rejected','cancelled')),
    notes            TEXT,
    approved_by      UUID        REFERENCES users(id) ON DELETE SET NULL,
    approved_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_date >= start_date)
);

CREATE INDEX leave_types_org      ON leave_types (organization_id);
CREATE INDEX leave_requests_org   ON leave_requests (organization_id);
CREATE INDEX leave_requests_emp   ON leave_requests (employee_id);
