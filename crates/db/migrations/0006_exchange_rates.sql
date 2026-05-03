CREATE TABLE exchange_rates (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    base_currency  CHAR(3)        NOT NULL,
    quote_currency CHAR(3)        NOT NULL,
    rate           FLOAT8         NOT NULL,  -- quote units per 1 base unit
    rate_date      DATE           NOT NULL,
    source         TEXT           NOT NULL,  -- e.g. "frankfurter", "manual"
    created_at     TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    UNIQUE (base_currency, quote_currency, rate_date)
);

CREATE INDEX idx_exchange_rates_lookup
    ON exchange_rates (base_currency, quote_currency, rate_date DESC);
