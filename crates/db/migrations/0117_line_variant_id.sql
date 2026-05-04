-- Allow invoice lines and bill lines to reference a specific product variant.
ALTER TABLE invoice_lines ADD COLUMN variant_id UUID REFERENCES product_variants(id) ON DELETE SET NULL;
ALTER TABLE bill_lines    ADD COLUMN variant_id UUID REFERENCES product_variants(id) ON DELETE SET NULL;

CREATE INDEX idx_invoice_lines_variant ON invoice_lines(variant_id) WHERE variant_id IS NOT NULL;
CREATE INDEX idx_bill_lines_variant    ON bill_lines(variant_id)    WHERE variant_id IS NOT NULL;
