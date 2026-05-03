ALTER TABLE journal_entries
    ADD COLUMN submitted_by UUID        REFERENCES users(id),
    ADD COLUMN submitted_at TIMESTAMPTZ,
    ADD COLUMN approved_by  UUID        REFERENCES users(id),
    ADD COLUMN approved_at  TIMESTAMPTZ;
