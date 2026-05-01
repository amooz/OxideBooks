-- OxideBooks initial schema (PostgreSQL)
--
-- Monetary values: INTEGER minor units (e.g. cents for USD).
-- IDs: native UUID type.
-- Timestamps: TIMESTAMPTZ (always UTC).

-- ─── Organizations ────────────────────────────────────────────────────────────

CREATE TABLE organizations (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name               TEXT NOT NULL,
    currency           TEXT NOT NULL DEFAULT 'USD',
    fiscal_year_start  SMALLINT NOT NULL DEFAULT 1
                           CHECK (fiscal_year_start BETWEEN 1 AND 12),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Users ────────────────────────────────────────────────────────────────────

CREATE TABLE users (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email            TEXT NOT NULL,
    password_hash    TEXT NOT NULL,
    name             TEXT NOT NULL,
    -- 'owner' | 'admin' | 'accountant' | 'viewer'
    role             TEXT NOT NULL DEFAULT 'viewer',
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, email)
);

-- ─── Chart of Accounts ────────────────────────────────────────────────────────

CREATE TABLE accounts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    code             TEXT NOT NULL,
    name             TEXT NOT NULL,
    -- 'asset' | 'liability' | 'equity' | 'revenue' | 'expense'
    account_type     TEXT NOT NULL,
    parent_id        UUID REFERENCES accounts(id) ON DELETE SET NULL,
    description      TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, code)
);

CREATE INDEX idx_accounts_org ON accounts(organization_id);

-- ─── Journal Entries ──────────────────────────────────────────────────────────

CREATE TABLE journal_entries (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    date             DATE NOT NULL,
    reference        TEXT,
    description      TEXT NOT NULL,
    -- 'draft' | 'posted' | 'voided'
    status           TEXT NOT NULL DEFAULT 'draft',
    created_by       UUID NOT NULL REFERENCES users(id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_journal_entries_org_date
    ON journal_entries(organization_id, date DESC);

-- ─── Journal Lines ────────────────────────────────────────────────────────────

CREATE TABLE journal_lines (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    journal_entry_id  UUID NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    account_id        UUID NOT NULL REFERENCES accounts(id),
    description       TEXT,
    debit             BIGINT NOT NULL DEFAULT 0 CHECK (debit >= 0),
    credit            BIGINT NOT NULL DEFAULT 0 CHECK (credit >= 0),
    CHECK (NOT (debit > 0 AND credit > 0))
);

CREATE INDEX idx_journal_lines_entry   ON journal_lines(journal_entry_id);
CREATE INDEX idx_journal_lines_account ON journal_lines(account_id);

-- ─── Contacts ─────────────────────────────────────────────────────────────────

CREATE TABLE contacts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    -- 'customer' | 'vendor' | 'both'
    contact_type     TEXT NOT NULL DEFAULT 'both',
    email            TEXT,
    phone            TEXT,
    address          TEXT,
    tax_number       TEXT,
    currency         TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contacts_org ON contacts(organization_id);

-- ─── Invoices ─────────────────────────────────────────────────────────────────

CREATE TABLE invoices (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id   UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invoice_number    TEXT NOT NULL,
    contact_id        UUID NOT NULL REFERENCES contacts(id),
    -- 'invoice' | 'bill'
    invoice_type      TEXT NOT NULL,
    -- 'draft' | 'sent' | 'partial' | 'paid' | 'overdue' | 'voided'
    status            TEXT NOT NULL DEFAULT 'draft',
    date              DATE NOT NULL,
    due_date          DATE NOT NULL,
    currency          TEXT NOT NULL DEFAULT 'USD',
    notes             TEXT,
    journal_entry_id  UUID REFERENCES journal_entries(id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, invoice_number)
);

CREATE INDEX idx_invoices_org_date ON invoices(organization_id, date DESC);
CREATE INDEX idx_invoices_contact  ON invoices(contact_id);

-- ─── Invoice Lines ────────────────────────────────────────────────────────────

CREATE TABLE invoice_lines (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id   UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    description  TEXT NOT NULL,
    account_id   UUID REFERENCES accounts(id),
    quantity     BIGINT NOT NULL DEFAULT 100  CHECK (quantity > 0),  -- qty × 100
    unit_price   BIGINT NOT NULL,
    tax_rate     BIGINT NOT NULL DEFAULT 0    CHECK (tax_rate >= 0), -- rate × 100
    sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_invoice_lines_invoice ON invoice_lines(invoice_id);
