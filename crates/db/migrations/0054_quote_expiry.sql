-- Add expiry_date for quotes and a declined/expired status transition.
ALTER TABLE invoices ADD COLUMN expiry_date DATE;
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_status_check;
ALTER TABLE invoices ADD CONSTRAINT invoices_status_check
    CHECK (status IN ('draft','sent','partial','paid','voided','accepted','declined','expired'));
