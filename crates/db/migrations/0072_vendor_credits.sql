-- Vendor credits (AP credit memos): credits from suppliers applied against bills
CREATE TABLE vendor_credits (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id      UUID REFERENCES contacts(id) ON DELETE SET NULL,
    credit_date     DATE NOT NULL,
    reference       TEXT,
    memo            TEXT,
    status          TEXT NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'partially_applied', 'fully_applied', 'voided')),
    total_amount    BIGINT NOT NULL DEFAULT 0 CHECK (total_amount >= 0),
    applied_amount  BIGINT NOT NULL DEFAULT 0 CHECK (applied_amount >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE vendor_credit_lines (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credit_id       UUID NOT NULL REFERENCES vendor_credits(id) ON DELETE CASCADE,
    account_id      UUID REFERENCES accounts(id) ON DELETE SET NULL,
    description     TEXT,
    quantity        BIGINT NOT NULL DEFAULT 100 CHECK (quantity > 0),
    unit_price      BIGINT NOT NULL CHECK (unit_price >= 0),
    tax_rate        BIGINT NOT NULL DEFAULT 0 CHECK (tax_rate >= 0),
    sort_order      INT NOT NULL DEFAULT 0
);

-- Tracks which vendor credit was applied to which bill
CREATE TABLE vendor_credit_applications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    credit_id       UUID NOT NULL REFERENCES vendor_credits(id) ON DELETE CASCADE,
    bill_id         UUID NOT NULL REFERENCES vendor_bills(id) ON DELETE CASCADE,
    amount          BIGINT NOT NULL CHECK (amount > 0),
    applied_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (credit_id, bill_id)
);

CREATE INDEX idx_vendor_credits_org     ON vendor_credits(organization_id);
CREATE INDEX idx_vendor_credit_lines    ON vendor_credit_lines(credit_id);
CREATE INDEX idx_vca_credit             ON vendor_credit_applications(credit_id);
CREATE INDEX idx_vca_bill               ON vendor_credit_applications(bill_id);
