-- Lease Accounting: ASC 842 / IFRS 16
-- Recognises Right-of-Use (ROU) assets and lease liabilities on the balance sheet.

CREATE TABLE leases (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name                 TEXT NOT NULL,
    description          TEXT,
    lease_type           TEXT NOT NULL
                             CHECK (lease_type IN ('finance', 'operating')),
    asset_account_id     UUID REFERENCES accounts(id) ON DELETE SET NULL,
    liability_account_id UUID REFERENCES accounts(id) ON DELETE SET NULL,
    expense_account_id   UUID REFERENCES accounts(id) ON DELETE SET NULL,
    commencement_date    DATE NOT NULL,
    end_date             DATE NOT NULL,
    -- Payment amount per period in minor units (e.g. cents)
    payment_amount       BIGINT NOT NULL CHECK (payment_amount > 0),
    payment_frequency    TEXT NOT NULL DEFAULT 'monthly'
                             CHECK (payment_frequency IN ('monthly','quarterly','annual')),
    -- Annual discount rate in basis points (e.g. 500 = 5.00%)
    discount_rate_bps    INT NOT NULL CHECK (discount_rate_bps >= 0),
    -- Computed at creation: PV of future lease payments
    initial_rou_asset    BIGINT NOT NULL DEFAULT 0,
    initial_liability    BIGINT NOT NULL DEFAULT 0,
    status               TEXT NOT NULL DEFAULT 'active'
                             CHECK (status IN ('active', 'terminated', 'expired')),
    terminated_at        DATE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Actual payments posted against a lease
CREATE TABLE lease_payments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    lease_id    UUID NOT NULL REFERENCES leases(id) ON DELETE CASCADE,
    period_date DATE NOT NULL,
    payment     BIGINT NOT NULL,
    interest    BIGINT NOT NULL DEFAULT 0,
    principal   BIGINT NOT NULL DEFAULT 0,
    rou_amort   BIGINT NOT NULL DEFAULT 0,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (lease_id, period_date)
);

CREATE INDEX idx_leases_org    ON leases(organization_id, status);
CREATE INDEX idx_lease_payments ON lease_payments(lease_id, period_date);
