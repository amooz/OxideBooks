CREATE TABLE invoice_templates (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id           UUID NOT NULL UNIQUE REFERENCES organizations(id) ON DELETE CASCADE,
    logo_url                  TEXT,
    accent_color              TEXT,
    footer_text               TEXT,
    default_payment_terms_days INT NOT NULL DEFAULT 30,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now()
);
