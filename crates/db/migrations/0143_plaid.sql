-- Plaid bank feed integration

-- Allow 'plaid' as a source in the existing feed table.
-- Recreate the CHECK constraint rather than ALTER (simpler for SQLite compat).
ALTER TABLE bank_feed_transactions
    DROP CONSTRAINT IF EXISTS bank_feed_transactions_source_check;

ALTER TABLE bank_feed_transactions
    ADD CONSTRAINT bank_feed_transactions_source_check
    CHECK (source IN ('csv', 'ofx', 'qfx', 'manual', 'plaid'));

-- Plaid items: one row per connected bank account via Plaid Link.
CREATE TABLE plaid_items (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    bank_account_id     UUID NOT NULL REFERENCES bank_accounts(id) ON DELETE CASCADE,
    item_id             TEXT NOT NULL,
    access_token        TEXT NOT NULL,
    institution_id      TEXT,
    institution_name    TEXT,
    -- Plaid transaction cursor (for incremental sync)
    cursor              TEXT,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    last_synced_at      TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX plaid_items_item_id_uidx ON plaid_items(item_id);
CREATE INDEX plaid_items_org ON plaid_items(organization_id);
CREATE INDEX plaid_items_account ON plaid_items(bank_account_id);

-- Track individual Plaid transaction IDs for deduplication.
CREATE TABLE plaid_transaction_ids (
    plaid_txn_id        TEXT PRIMARY KEY,
    feed_txn_id         UUID NOT NULL REFERENCES bank_feed_transactions(id) ON DELETE CASCADE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
