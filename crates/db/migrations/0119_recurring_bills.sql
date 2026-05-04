-- Recurring vendor bill templates — auto-generate draft bills on a schedule
CREATE TABLE recurring_bills (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id          TEXT        NOT NULL,
    description         TEXT        NOT NULL,
    reference           TEXT,
    currency_code       TEXT        NOT NULL DEFAULT 'USD',
    frequency           TEXT        NOT NULL CHECK (frequency IN ('weekly','monthly','quarterly','yearly')),
    interval_count      INT         NOT NULL DEFAULT 1 CHECK (interval_count >= 1),
    next_due_date       DATE        NOT NULL,
    end_date            DATE,
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    days_due            INT         NOT NULL DEFAULT 30,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE recurring_bill_lines (
    id                  UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    recurring_bill_id   UUID    NOT NULL REFERENCES recurring_bills(id) ON DELETE CASCADE,
    description         TEXT    NOT NULL,
    quantity            INT     NOT NULL DEFAULT 1,
    unit_price          BIGINT  NOT NULL,
    account_id          TEXT,
    tax_rate            BIGINT  NOT NULL DEFAULT 0,
    sort_order          INT     NOT NULL DEFAULT 0
);

CREATE INDEX idx_recurring_bills_org        ON recurring_bills(organization_id, is_active);
CREATE INDEX idx_recurring_bills_next_due   ON recurring_bills(next_due_date) WHERE is_active = TRUE;
CREATE INDEX idx_recurring_bill_lines_rb    ON recurring_bill_lines(recurring_bill_id);
