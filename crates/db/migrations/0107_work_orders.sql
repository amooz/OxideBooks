-- Work orders (service tickets) for service-business workflows
CREATE TABLE work_orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id      UUID REFERENCES contacts(id),
    assigned_to     UUID REFERENCES users(id),
    title           TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'open'
                        CHECK (status IN ('open','in_progress','on_hold','completed','invoiced','cancelled')),
    priority        TEXT NOT NULL DEFAULT 'normal'
                        CHECK (priority IN ('low','normal','high','urgent')),
    scheduled_date  DATE,
    completed_date  DATE,
    invoice_id      UUID REFERENCES invoices(id),
    doc_number      TEXT,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE work_order_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    work_order_id   UUID NOT NULL REFERENCES work_orders(id) ON DELETE CASCADE,
    product_id      UUID REFERENCES products(id),
    description     TEXT NOT NULL DEFAULT '',
    quantity        INT  NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price      BIGINT NOT NULL DEFAULT 0 CHECK (unit_price >= 0),
    completed       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_work_orders_org     ON work_orders(organization_id, status);
CREATE INDEX idx_work_orders_contact ON work_orders(contact_id) WHERE contact_id IS NOT NULL;
