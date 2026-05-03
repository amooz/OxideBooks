CREATE TABLE tags (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    color           TEXT        NOT NULL DEFAULT '#6366f1',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, name)
);

CREATE TABLE entity_tags (
    tag_id      UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    entity_id   UUID NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('contact','invoice','expense','project','purchase_order')),
    PRIMARY KEY (tag_id, entity_id)
);

CREATE INDEX idx_tags_org        ON tags (organization_id);
CREATE INDEX idx_entity_tags_eid ON entity_tags (entity_id);
CREATE INDEX idx_entity_tags_tag ON entity_tags (tag_id);
