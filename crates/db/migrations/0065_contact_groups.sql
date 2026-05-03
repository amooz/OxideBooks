CREATE TABLE contact_groups (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE contact_group_members (
    group_id   UUID NOT NULL REFERENCES contact_groups(id)  ON DELETE CASCADE,
    contact_id UUID NOT NULL REFERENCES contacts(id)        ON DELETE CASCADE,
    added_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, contact_id)
);

CREATE INDEX idx_contact_group_members_contact ON contact_group_members(contact_id);
