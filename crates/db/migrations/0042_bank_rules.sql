CREATE TABLE bank_rules (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    match_field      TEXT NOT NULL CHECK (match_field IN ('description', 'amount')),
    match_type       TEXT NOT NULL CHECK (match_type IN ('contains', 'equals', 'gt', 'lt')),
    match_value      TEXT NOT NULL,
    account_id       UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    auto_description TEXT,
    priority         INT NOT NULL DEFAULT 100,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX bank_rules_org ON bank_rules (organization_id, priority);
