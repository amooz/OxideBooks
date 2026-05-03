ALTER TABLE invoice_lines ADD COLUMN product_id UUID REFERENCES products(id) ON DELETE SET NULL;

CREATE INDEX idx_invoice_lines_product ON invoice_lines(product_id) WHERE product_id IS NOT NULL;
