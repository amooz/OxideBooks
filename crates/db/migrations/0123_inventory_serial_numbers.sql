CREATE TABLE inventory_serial_numbers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id),
    product_id          UUID NOT NULL REFERENCES products(id),
    serial_number       TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'available'
                            CHECK (status IN ('available','sold','returned','scrapped')),
    lot_id              UUID REFERENCES inventory_lots(id),
    warehouse_id        UUID REFERENCES warehouses(id),
    purchase_date       DATE,
    sold_date           DATE,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, product_id, serial_number)
);

CREATE INDEX ON inventory_serial_numbers (organization_id, product_id);
CREATE INDEX ON inventory_serial_numbers (organization_id, status);
