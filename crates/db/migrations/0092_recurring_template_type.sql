ALTER TABLE recurring_schedules
    ADD COLUMN template_type TEXT NOT NULL DEFAULT 'invoice'
        CHECK (template_type IN ('invoice', 'bill'));
