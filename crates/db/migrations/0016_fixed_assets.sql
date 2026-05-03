CREATE TABLE fixed_assets (
    id                              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id                 UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name                            TEXT        NOT NULL,
    asset_number                    TEXT        NOT NULL,
    purchase_date                   DATE        NOT NULL,
    purchase_cost                   BIGINT      NOT NULL CHECK (purchase_cost >= 0),
    salvage_value                   BIGINT      NOT NULL DEFAULT 0,
    useful_life_months              INT         NOT NULL CHECK (useful_life_months > 0),
    depreciation_method             TEXT        NOT NULL DEFAULT 'straight_line'
                                    CHECK (depreciation_method IN ('straight_line','declining_balance')),
    asset_account_id                UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    accumulated_depreciation_acct   UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    depreciation_expense_acct       UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    status                          TEXT        NOT NULL DEFAULT 'active'
                                    CHECK (status IN ('active','disposed')),
    disposed_at                     DATE,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE asset_depreciation_entries (
    id              UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID    NOT NULL REFERENCES fixed_assets(id) ON DELETE CASCADE,
    period_date     DATE    NOT NULL,
    amount          BIGINT  NOT NULL,
    journal_entry_id UUID   REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, period_date)
);

CREATE INDEX idx_fixed_assets_org ON fixed_assets (organization_id);
CREATE INDEX idx_asset_dep_asset  ON asset_depreciation_entries (asset_id);
