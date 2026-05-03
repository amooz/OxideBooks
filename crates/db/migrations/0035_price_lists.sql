CREATE TABLE price_lists (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    currency        TEXT NOT NULL DEFAULT 'USD',
    is_default      BOOL NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE price_list_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    price_list_id   UUID NOT NULL REFERENCES price_lists(id) ON DELETE CASCADE,
    product_id      UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    unit_price      BIGINT NOT NULL CHECK (unit_price >= 0),
    UNIQUE (price_list_id, product_id)
);

CREATE INDEX idx_price_lists_org  ON price_lists(organization_id);
CREATE INDEX idx_price_list_items ON price_list_items(price_list_id);
