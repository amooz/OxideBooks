-- Tracks whether an opening-balance journal entry has been set for an org.
-- The actual debits/credits live in journal_entries / journal_lines as usual.
ALTER TABLE organizations ADD COLUMN opening_balance_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL;
