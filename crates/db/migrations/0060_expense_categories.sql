CREATE TABLE expense_categories (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT        NOT NULL,
    description      TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

ALTER TABLE expenses
    ADD COLUMN expense_category_id UUID REFERENCES expense_categories(id) ON DELETE SET NULL;

CREATE INDEX expense_categories_org ON expense_categories (organization_id);
CREATE INDEX expenses_category ON expenses (expense_category_id);
