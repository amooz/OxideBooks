-- Period-end unrealized FX revaluation entries.
-- Each run snapshots the gain/loss on open AR/AP items at a given exchange rate.
CREATE TABLE fx_revaluations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    revaluation_date DATE NOT NULL,
    currency        TEXT NOT NULL,
    rate            NUMERIC(20, 10) NOT NULL,
    net_gain_loss   BIGINT NOT NULL,
    journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_fxrev_org  ON fx_revaluations(organization_id);
CREATE INDEX idx_fxrev_date ON fx_revaluations(organization_id, revaluation_date DESC);
