CREATE TABLE product_categories (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT        NOT NULL,
    description      TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

ALTER TABLE products
    ADD COLUMN category_id UUID REFERENCES product_categories(id) ON DELETE SET NULL;

CREATE INDEX product_categories_org ON product_categories (organization_id);
CREATE INDEX products_category ON products (category_id);
