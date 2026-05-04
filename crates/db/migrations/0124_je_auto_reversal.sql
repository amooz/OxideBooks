-- Support scheduled auto-reversal for accrual journal entries.
-- When auto_reversal_date is set and the entry hasn't been reversed yet,
-- the POST /transactions/auto-reversals batch endpoint will create the reversal.
ALTER TABLE journal_entries
    ADD COLUMN auto_reversal_date DATE DEFAULT NULL;
