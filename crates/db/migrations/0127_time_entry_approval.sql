ALTER TABLE time_entries
    ADD COLUMN approval_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (approval_status IN ('pending', 'approved', 'rejected')),
    ADD COLUMN approved_by    UUID        REFERENCES users(id),
    ADD COLUMN approved_at    TIMESTAMPTZ,
    ADD COLUMN rejection_reason TEXT;

CREATE INDEX idx_time_entries_approval ON time_entries (organization_id, approval_status);
