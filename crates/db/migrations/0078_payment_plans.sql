-- Payment plans: break an invoice into scheduled installments
CREATE TABLE payment_plans (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_id      UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    contact_id      UUID NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    description     TEXT,
    total_amount    BIGINT NOT NULL CHECK (total_amount > 0),
    paid_amount     BIGINT NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','completed','cancelled')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE payment_plan_installments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id     UUID NOT NULL REFERENCES payment_plans(id) ON DELETE CASCADE,
    due_date    DATE NOT NULL,
    amount      BIGINT NOT NULL CHECK (amount > 0),
    paid_amount BIGINT NOT NULL DEFAULT 0,
    status      TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending','partial','paid','overdue')),
    sort_order  INT NOT NULL DEFAULT 0
);

CREATE INDEX idx_pplan_org     ON payment_plans(organization_id);
CREATE INDEX idx_pplan_invoice ON payment_plans(invoice_id);
CREATE INDEX idx_pplan_inst    ON payment_plan_installments(plan_id);
