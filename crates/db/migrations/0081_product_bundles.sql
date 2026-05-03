-- Product bundles: composite products made of component items
ALTER TABLE products ADD COLUMN is_bundle BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE product_bundle_components (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id      UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    component_id    UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    quantity        BIGINT NOT NULL DEFAULT 100 CHECK (quantity > 0),
    sort_order      INT NOT NULL DEFAULT 0,
    UNIQUE (product_id, component_id)
);

CREATE INDEX idx_pbc_product   ON product_bundle_components(product_id);
CREATE INDEX idx_pbc_component ON product_bundle_components(component_id);
