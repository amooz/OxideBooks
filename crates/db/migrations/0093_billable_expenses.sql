-- Billable expenses: mark an expense as billable to a specific contact
-- so it can later be included on an invoice.
ALTER TABLE expenses
    ADD COLUMN is_billable           BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN billable_contact_id   UUID    REFERENCES contacts(id),
    ADD COLUMN billed_invoice_id     UUID    REFERENCES invoices(id);

CREATE INDEX idx_expenses_billable
    ON expenses(organization_id, billable_contact_id)
    WHERE is_billable = TRUE AND billed_invoice_id IS NULL;
