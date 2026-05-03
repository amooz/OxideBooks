-- Add credit limit support to contacts.
-- credit_limit = NULL means no limit enforced.
-- credit_limit_behaviour: 'warn' (return warning but allow) or 'block' (reject if exceeded).
ALTER TABLE contacts
    ADD COLUMN credit_limit          BIGINT,
    ADD COLUMN credit_limit_behaviour TEXT NOT NULL DEFAULT 'warn'
        CHECK (credit_limit_behaviour IN ('warn', 'block'));
