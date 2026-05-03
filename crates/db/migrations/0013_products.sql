CREATE TABLE products (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    description     TEXT,
    sku             TEXT,
    unit_price      BIGINT      NOT NULL DEFAULT 0,
    currency        TEXT        NOT NULL DEFAULT 'USD',
    account_id      UUID        REFERENCES accounts(id) ON DELETE SET NULL,
    tax_rate_id     UUID        REFERENCES tax_rates(id) ON DELETE SET NULL,
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_products_org ON products (organization_id);
CREATE UNIQUE INDEX idx_products_sku ON products (organization_id, sku) WHERE sku IS NOT NULL;
