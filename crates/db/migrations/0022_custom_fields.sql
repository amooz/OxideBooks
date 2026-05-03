CREATE TABLE custom_field_definitions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    entity_type     TEXT        NOT NULL CHECK (entity_type IN ('contact','invoice','expense','project')),
    name            TEXT        NOT NULL,
    field_type      TEXT        NOT NULL CHECK (field_type IN ('text','number','date','boolean')),
    is_required     BOOLEAN     NOT NULL DEFAULT FALSE,
    sort_order      INT         NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, entity_type, name)
);

CREATE TABLE custom_field_values (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    definition_id   UUID        NOT NULL REFERENCES custom_field_definitions(id) ON DELETE CASCADE,
    entity_id       UUID        NOT NULL,
    value           TEXT,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (definition_id, entity_id)
);

CREATE INDEX idx_cfdef_org    ON custom_field_definitions (organization_id, entity_type);
CREATE INDEX idx_cfval_entity ON custom_field_values (entity_id);
