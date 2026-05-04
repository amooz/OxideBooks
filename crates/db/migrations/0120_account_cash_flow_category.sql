-- Add cash flow classification to accounts for indirect cash flow statement
ALTER TABLE accounts
    ADD COLUMN cash_flow_category TEXT
        CHECK (cash_flow_category IN ('operating','investing','financing'))
        DEFAULT NULL;

COMMENT ON COLUMN accounts.cash_flow_category IS
    'Cash flow statement classification: operating, investing, financing, or NULL (non-cash/balance sheet).';
