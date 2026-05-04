-- Realized FX gain/loss: store the computed difference on each payment
ALTER TABLE payments
    ADD COLUMN realized_fx_amount BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN fx_journal_entry_id UUID REFERENCES journal_entries(id);
