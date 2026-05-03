-- Add domain for SSO provider discovery (#14)
ALTER TABLE organizations ADD COLUMN domain TEXT UNIQUE;
