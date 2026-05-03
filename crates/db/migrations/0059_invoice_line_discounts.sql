ALTER TABLE invoice_lines
    ADD COLUMN discount_pct BIGINT NOT NULL DEFAULT 0 CHECK (discount_pct >= 0 AND discount_pct <= 10000);
