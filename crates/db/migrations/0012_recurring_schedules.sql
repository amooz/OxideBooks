CREATE TABLE recurring_schedules (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- Template stored as JSONB (CreateInvoice payload minus invoice_number)
    template        JSONB       NOT NULL,
    frequency       TEXT        NOT NULL CHECK (frequency IN ('weekly','monthly','quarterly','yearly')),
    interval_count  INT         NOT NULL DEFAULT 1 CHECK (interval_count >= 1),
    next_due_date   DATE        NOT NULL,
    end_date        DATE,
    auto_send       BOOLEAN     NOT NULL DEFAULT FALSE,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_recurring_org        ON recurring_schedules (organization_id);
CREATE INDEX idx_recurring_due        ON recurring_schedules (next_due_date) WHERE is_active = TRUE;
