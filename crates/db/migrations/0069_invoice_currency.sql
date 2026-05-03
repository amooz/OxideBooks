-- Add exchange_rate to invoices (currency column already exists)
ALTER TABLE invoices
    ADD COLUMN exchange_rate NUMERIC(18, 8) NOT NULL DEFAULT 1;

-- Add currency and exchange_rate to vendor_bills
ALTER TABLE vendor_bills
    ADD COLUMN currency_code  TEXT NOT NULL DEFAULT 'USD',
    ADD COLUMN exchange_rate  NUMERIC(18, 8) NOT NULL DEFAULT 1;

-- Add doc_number to key documents for custom numbering sequences
ALTER TABLE invoices      ADD COLUMN doc_number TEXT;
ALTER TABLE vendor_bills  ADD COLUMN doc_number TEXT;
ALTER TABLE credit_notes  ADD COLUMN doc_number TEXT;
ALTER TABLE purchase_orders ADD COLUMN doc_number TEXT;

CREATE UNIQUE INDEX idx_invoices_doc_number
    ON invoices(organization_id, doc_number) WHERE doc_number IS NOT NULL;
CREATE UNIQUE INDEX idx_vendor_bills_doc_number
    ON vendor_bills(organization_id, doc_number) WHERE doc_number IS NOT NULL;
CREATE UNIQUE INDEX idx_credit_notes_doc_number
    ON credit_notes(organization_id, doc_number) WHERE doc_number IS NOT NULL;
CREATE UNIQUE INDEX idx_po_doc_number
    ON purchase_orders(organization_id, doc_number) WHERE doc_number IS NOT NULL;
