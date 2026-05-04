-- Link vendor bills back to the purchase order they were created from.
-- Enables 3-way matching (PO → GRN → Bill) and GRNI accrual reporting.
ALTER TABLE vendor_bills ADD COLUMN purchase_order_id UUID REFERENCES purchase_orders(id);

CREATE INDEX idx_vendor_bills_po ON vendor_bills(purchase_order_id)
    WHERE purchase_order_id IS NOT NULL;
