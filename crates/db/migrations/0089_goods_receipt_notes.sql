CREATE TABLE goods_receipt_notes (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    purchase_order_id   UUID        NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    receipt_date        DATE        NOT NULL DEFAULT CURRENT_DATE,
    reference           TEXT,
    notes               TEXT,
    status              TEXT        NOT NULL DEFAULT 'draft'
                            CHECK (status IN ('draft', 'posted')),
    created_by          TEXT        NOT NULL DEFAULT '',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE grn_lines (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    grn_id              UUID        NOT NULL REFERENCES goods_receipt_notes(id) ON DELETE CASCADE,
    po_line_id          UUID        NOT NULL REFERENCES purchase_order_lines(id),
    item_id             UUID        REFERENCES inventory_items(id),
    lot_id              UUID        REFERENCES inventory_lots(id),
    quantity_received   BIGINT      NOT NULL CHECK (quantity_received > 0),
    unit_cost           BIGINT      NOT NULL DEFAULT 0
);

CREATE INDEX ON goods_receipt_notes (organization_id);
CREATE INDEX ON goods_receipt_notes (purchase_order_id);
