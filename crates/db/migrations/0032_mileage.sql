CREATE TABLE mileage_trips (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id),
    trip_date       DATE NOT NULL,
    distance_km     NUMERIC(10,2) NOT NULL CHECK (distance_km > 0),
    purpose         TEXT NOT NULL,
    rate_per_km     BIGINT NOT NULL DEFAULT 0 CHECK (rate_per_km >= 0),
    reimbursable    BOOL NOT NULL DEFAULT true,
    expense_id      UUID REFERENCES expenses(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_mileage_trips_org ON mileage_trips(organization_id, trip_date DESC);
CREATE INDEX idx_mileage_trips_user ON mileage_trips(user_id);
