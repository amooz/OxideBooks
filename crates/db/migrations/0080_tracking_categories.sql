-- Tracking categories: QB-style classes/locations for multi-dimensional reporting
CREATE TABLE tracking_categories (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE tracking_options (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES tracking_categories(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (category_id, name)
);

-- Allow tagging invoice lines with tracking options
CREATE TABLE invoice_line_tracking (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    line_id     UUID NOT NULL REFERENCES invoice_lines(id) ON DELETE CASCADE,
    option_id   UUID NOT NULL REFERENCES tracking_options(id) ON DELETE CASCADE,
    UNIQUE (line_id, option_id)
);

-- Allow tagging journal lines with tracking options
CREATE TABLE journal_line_tracking (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    line_id     UUID NOT NULL REFERENCES journal_lines(id) ON DELETE CASCADE,
    option_id   UUID NOT NULL REFERENCES tracking_options(id) ON DELETE CASCADE,
    UNIQUE (line_id, option_id)
);

CREATE INDEX idx_tc_org         ON tracking_categories(organization_id);
CREATE INDEX idx_to_category    ON tracking_options(category_id);
CREATE INDEX idx_ilt_line       ON invoice_line_tracking(line_id);
CREATE INDEX idx_jlt_line       ON journal_line_tracking(line_id);
