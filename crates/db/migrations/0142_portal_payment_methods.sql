-- Portal saved payment methods and autopay enrollment

CREATE TABLE portal_payment_methods (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id   UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id        UUID NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    payment_type      TEXT NOT NULL CHECK (payment_type IN ('card', 'bank_account', 'paypal')),
    provider          TEXT NOT NULL DEFAULT 'stripe',
    provider_token    TEXT NOT NULL,
    last4             TEXT,
    brand             TEXT,
    exp_month         SMALLINT,
    exp_year          SMALLINT,
    bank_name         TEXT,
    is_default        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON portal_payment_methods(organization_id, contact_id);

-- Only one default per (org, contact)
CREATE UNIQUE INDEX portal_pm_default_uidx
    ON portal_payment_methods(organization_id, contact_id)
    WHERE is_default = TRUE;

CREATE TABLE portal_autopay_enrollments (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id          UUID NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    payment_method_id   UUID NOT NULL REFERENCES portal_payment_methods(id) ON DELETE CASCADE,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    days_before_due     INT NOT NULL DEFAULT 0 CHECK (days_before_due >= 0),
    max_amount          BIGINT CHECK (max_amount IS NULL OR max_amount > 0),
    enrolled_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ON portal_autopay_enrollments(organization_id, contact_id);

-- At most one active enrollment per contact
CREATE UNIQUE INDEX portal_autopay_active_uidx
    ON portal_autopay_enrollments(organization_id, contact_id)
    WHERE is_active = TRUE;
