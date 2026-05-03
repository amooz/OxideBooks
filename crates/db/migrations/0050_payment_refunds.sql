ALTER TABLE payments ADD COLUMN status TEXT NOT NULL DEFAULT 'recorded';
ALTER TABLE payments ADD COLUMN voided_at TIMESTAMPTZ;

CREATE TABLE refunds (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id   UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    amount       BIGINT NOT NULL,
    reason       TEXT,
    refund_date  DATE NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX refunds_payment ON refunds (payment_id);
