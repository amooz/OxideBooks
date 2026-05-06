ALTER TABLE payslips
    ADD COLUMN status       TEXT        NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'published')),
    ADD COLUMN published_at TIMESTAMPTZ;
