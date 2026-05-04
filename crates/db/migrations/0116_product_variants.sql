-- Product variants: allow a single product to have multiple SKU-level
-- variations (e.g. size, colour) with optional price overrides.
CREATE TABLE product_variants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id      UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    sku             TEXT,
    name            TEXT NOT NULL,
    -- Free-form key/value attributes (e.g. {"size":"M","color":"Red"})
    attributes      JSONB NOT NULL DEFAULT '{}',
    -- NULL means "use the parent product's unit_price"
    price_override  BIGINT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (product_id, sku) DEFERRABLE INITIALLY IMMEDIATE
);

CREATE INDEX idx_product_variants_product ON product_variants(product_id, is_active);
CREATE UNIQUE INDEX idx_product_variants_sku
    ON product_variants(organization_id, sku)
    WHERE sku IS NOT NULL;
