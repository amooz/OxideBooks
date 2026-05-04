-- reorder_point already exists; add reorder_qty for suggested order quantity.
ALTER TABLE inventory_items
    ADD COLUMN reorder_qty BIGINT NOT NULL DEFAULT 0 CHECK (reorder_qty >= 0);
