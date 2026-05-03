-- Global (header-level) discount applied to invoice subtotal before tax.
-- Value is integer basis points × 100 (e.g. 10% → 1000, same scale as line discount_pct).
ALTER TABLE invoices
    ADD COLUMN global_discount_pct BIGINT NOT NULL DEFAULT 0
        CHECK (global_discount_pct >= 0 AND global_discount_pct <= 10000);
