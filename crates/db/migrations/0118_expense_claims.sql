-- Expense claims: employee-submitted requests for reimbursement
CREATE TABLE expense_claims (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    claimant_id         TEXT        NOT NULL,
    title               TEXT        NOT NULL,
    description         TEXT,
    status              TEXT        NOT NULL DEFAULT 'draft'
                            CHECK (status IN ('draft','submitted','approved','rejected','reimbursed')),
    submitted_at        TIMESTAMPTZ,
    reviewed_at         TIMESTAMPTZ,
    reviewer_id         TEXT,
    reviewer_notes      TEXT,
    reimbursed_at       TIMESTAMPTZ,
    currency_code       TEXT        NOT NULL DEFAULT 'USD',
    total_amount        BIGINT      NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE expense_claim_lines (
    id              UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    claim_id        UUID    NOT NULL REFERENCES expense_claims(id) ON DELETE CASCADE,
    date            DATE    NOT NULL,
    description     TEXT    NOT NULL,
    amount          BIGINT  NOT NULL,
    category        TEXT,
    receipt_url     TEXT,
    account_id      TEXT,
    sort_order      INT     NOT NULL DEFAULT 0
);

CREATE INDEX idx_expense_claims_org_status   ON expense_claims(organization_id, status);
CREATE INDEX idx_expense_claims_claimant     ON expense_claims(organization_id, claimant_id);
CREATE INDEX idx_expense_claim_lines_claim   ON expense_claim_lines(claim_id);
