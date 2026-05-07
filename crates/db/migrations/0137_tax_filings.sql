CREATE TABLE tax_filings (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id   UUID NOT NULL REFERENCES organizations(id),
    filing_type       TEXT NOT NULL
                          CHECK (filing_type IN
                              ('1099_nec','1099_misc','w2','941','t4','t4a','hst_gst')),
    period_year       INT NOT NULL,
    period_quarter    INT,            -- NULL for annual/period filings
    period_from       DATE,           -- for HST/GST range filings
    period_to         DATE,
    tax_jurisdiction  TEXT NOT NULL DEFAULT 'us_federal'
                          CHECK (tax_jurisdiction IN
                              ('us_federal','us_state','ca_federal','ca_provincial')),
    status            TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft','submitted','accepted')),
    -- JSON blob of the filing payload (slips array, totals, etc.)
    summary_data      JSONB,
    -- CRA XML or IRS EFW2 output stored here for e-file download
    efile_xml         TEXT,
    submitted_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tax_filings_org  ON tax_filings (organization_id);
CREATE INDEX idx_tax_filings_type ON tax_filings (organization_id, filing_type, period_year);
