CREATE TABLE dunning_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    days_overdue    INT NOT NULL CHECK (days_overdue > 0),
    reminder_level  INT NOT NULL DEFAULT 1,
    subject_template TEXT NOT NULL DEFAULT 'Invoice overdue reminder',
    body_template   TEXT NOT NULL DEFAULT 'Your invoice is overdue.',
    is_active       BOOL NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, days_overdue)
);

CREATE TABLE invoice_reminders (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id  UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    rule_id     UUID REFERENCES dunning_rules(id) ON DELETE SET NULL,
    sent_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    to_address  TEXT NOT NULL,
    level       INT NOT NULL DEFAULT 1
);

CREATE INDEX idx_invoice_reminders_invoice ON invoice_reminders(invoice_id);
