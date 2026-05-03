CREATE TABLE user_sessions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID        NOT NULL,
    organization_id UUID        NOT NULL,
    jti             TEXT        NOT NULL UNIQUE,
    user_agent      TEXT,
    ip              TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at      TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON user_sessions(jti);
CREATE INDEX ON user_sessions(user_id);
