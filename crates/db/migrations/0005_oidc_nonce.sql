-- Persist the OIDC nonce so it can be verified in the callback.
-- Without this, the nonce field is discarded at initiation and an empty string
-- is used for verification, defeating the nonce's replay-protection purpose.
ALTER TABLE oidc_states ADD COLUMN nonce TEXT NOT NULL DEFAULT '';
