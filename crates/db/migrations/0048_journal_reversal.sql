ALTER TABLE journal_entries ADD COLUMN reversal_of UUID REFERENCES journal_entries(id) ON DELETE SET NULL;
CREATE INDEX journal_entries_reversal_of ON journal_entries (reversal_of) WHERE reversal_of IS NOT NULL;
