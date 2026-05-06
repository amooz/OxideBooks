ALTER TABLE inventory_items
    ADD COLUMN quantity_reserved BIGINT NOT NULL DEFAULT 0 CHECK (quantity_reserved >= 0);
