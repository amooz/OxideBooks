-- Add billing method to projects
ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS billing_method TEXT NOT NULL DEFAULT 'time_and_materials'
        CHECK (billing_method IN ('fixed_fee', 'time_and_materials', 'milestone', 'progress'));

-- Progress claims for progress-billing projects
CREATE TABLE project_progress_claims (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id   UUID NOT NULL REFERENCES organizations(id),
    project_id        UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    claim_number      INT NOT NULL,
    -- Cumulative % claimed this draw (stored × 100; e.g. 25% → 2500)
    -- 0 is reserved for retainage-release rows; regular claims must use > 0 (enforced in app)
    claim_pct         BIGINT NOT NULL CHECK (claim_pct >= 0 AND claim_pct <= 10000),
    claim_amount      BIGINT NOT NULL CHECK (claim_amount >= 0),
    -- Retention holdback % (stored × 100)
    retainage_pct     BIGINT NOT NULL DEFAULT 0 CHECK (retainage_pct >= 0),
    retainage_amount  BIGINT NOT NULL DEFAULT 0,
    net_amount        BIGINT NOT NULL,   -- claim_amount - retainage_amount
    status            TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft', 'approved', 'invoiced')),
    notes             TEXT,
    invoice_id        UUID REFERENCES invoices(id),
    approved_at       TIMESTAMPTZ,
    invoiced_at       TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, claim_number)
);

CREATE INDEX idx_progress_claims_project ON project_progress_claims (project_id);
CREATE INDEX idx_progress_claims_org     ON project_progress_claims (organization_id);
