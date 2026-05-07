-- Multi-level approval chains extending the existing approval_rules table.

CREATE TABLE approval_chains (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    entity_type     TEXT NOT NULL CHECK (entity_type IN (
                        'expense', 'bill', 'purchase_order',
                        'purchase_requisition', 'journal_entry', 'payment'
                    )),
    description     TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE approval_chain_steps (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id         UUID NOT NULL REFERENCES approval_chains(id) ON DELETE CASCADE,
    step_order       INT NOT NULL,
    required_role    TEXT NOT NULL DEFAULT 'accountant'
                         CHECK (required_role IN ('accountant', 'admin', 'owner')),
    -- If set, this specific user must approve; otherwise any user with the role.
    approver_user_id UUID REFERENCES users(id),
    -- If true, all users with the role must approve (consensus).
    require_all      BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (chain_id, step_order)
);

CREATE TYPE approval_status AS ENUM ('pending', 'approved', 'rejected', 'cancelled');

CREATE TABLE approval_requests (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    chain_id         UUID NOT NULL REFERENCES approval_chains(id),
    entity_type      TEXT NOT NULL,
    entity_id        UUID NOT NULL,
    status           approval_status NOT NULL DEFAULT 'pending',
    current_step     INT NOT NULL DEFAULT 1,
    requested_by     UUID REFERENCES users(id),
    notes            TEXT,
    completed_at     TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE approval_decisions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id       UUID NOT NULL REFERENCES approval_requests(id) ON DELETE CASCADE,
    step_order       INT NOT NULL,
    approver_user_id UUID REFERENCES users(id),
    decision         TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
    notes            TEXT,
    decided_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX approval_chains_org_idx ON approval_chains(organization_id);
CREATE INDEX approval_requests_org_idx ON approval_requests(organization_id);
CREATE INDEX approval_requests_entity_idx ON approval_requests(entity_type, entity_id);
CREATE INDEX approval_decisions_request_idx ON approval_decisions(request_id);
