-- Purchase requisitions (internal requests that become POs after approval)
CREATE TABLE purchase_requisitions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    requester_id    UUID REFERENCES users(id) ON DELETE SET NULL,
    approver_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    notes           TEXT,
    status          TEXT NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'submitted', 'approved', 'rejected', 'converted')),
    total_amount    BIGINT NOT NULL DEFAULT 0,
    approved_at     TIMESTAMPTZ,
    rejected_at     TIMESTAMPTZ,
    converted_po_id UUID REFERENCES purchase_orders(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE purchase_requisition_lines (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requisition_id   UUID NOT NULL REFERENCES purchase_requisitions(id) ON DELETE CASCADE,
    product_id       UUID REFERENCES products(id) ON DELETE SET NULL,
    description      TEXT NOT NULL,
    quantity         BIGINT NOT NULL CHECK (quantity > 0),
    unit_price       BIGINT NOT NULL CHECK (unit_price >= 0),
    account_id       UUID REFERENCES accounts(id) ON DELETE SET NULL,
    sort_order       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_pr_org     ON purchase_requisitions(organization_id);
CREATE INDEX idx_pr_lines   ON purchase_requisition_lines(requisition_id);
