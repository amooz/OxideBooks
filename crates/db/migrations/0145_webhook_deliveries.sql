-- Webhook delivery log with retry support

CREATE TABLE webhook_deliveries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    endpoint_id     UUID NOT NULL REFERENCES webhook_endpoints(id) ON DELETE CASCADE,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'success', 'failed', 'retrying')),
    http_status     INT,
    response_body   TEXT,
    attempt_count   INT NOT NULL DEFAULT 0,
    next_retry_at   TIMESTAMPTZ,
    delivered_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX wh_deliveries_endpoint  ON webhook_deliveries(endpoint_id, created_at DESC);
CREATE INDEX wh_deliveries_org       ON webhook_deliveries(organization_id, created_at DESC);
CREATE INDEX wh_deliveries_retry     ON webhook_deliveries(next_retry_at)
    WHERE status IN ('pending', 'retrying');
