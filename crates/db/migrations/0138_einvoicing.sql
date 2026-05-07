CREATE TYPE einvoice_format AS ENUM ('ubl', 'peppol');
CREATE TYPE einvoice_status AS ENUM ('pending', 'sent', 'acknowledged', 'rejected');

CREATE TABLE einvoice_transmissions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID NOT NULL REFERENCES organizations(id),
    invoice_id       UUID NOT NULL REFERENCES invoices(id),
    format           einvoice_format NOT NULL DEFAULT 'ubl',
    status           einvoice_status NOT NULL DEFAULT 'pending',
    external_id      TEXT,
    transmission_xml TEXT,
    error_message    TEXT,
    sent_at          TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX einvoice_transmissions_org_idx ON einvoice_transmissions(organization_id);
CREATE INDEX einvoice_transmissions_invoice_idx ON einvoice_transmissions(invoice_id);
