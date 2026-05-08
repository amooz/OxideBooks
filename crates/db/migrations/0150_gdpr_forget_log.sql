-- GDPR compliance: track contact anonymization requests (immutable log)
CREATE TABLE gdpr_forget_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id      UUID NOT NULL,
    requested_by    UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymized_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gdpr_forget_org ON gdpr_forget_requests(organization_id, anonymized_at DESC);
