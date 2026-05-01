-- RBAC: permissions, roles, role_permissions; migrate users.role → users.role_id
--
-- System roles use well-known UUIDs so the application can reference them
-- without a lookup. Org-custom roles are created at runtime.

-- ─── Permissions catalog (global, seeded once) ────────────────────────────────

CREATE TABLE permissions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT
);

INSERT INTO permissions (name, description) VALUES
    ('accounts:read',      'View chart of accounts'),
    ('accounts:write',     'Create and update accounts'),
    ('accounts:delete',    'Delete accounts'),
    ('transactions:read',  'View journal entries'),
    ('transactions:write', 'Create journal entries'),
    ('contacts:read',      'View contacts'),
    ('contacts:write',     'Create and update contacts'),
    ('invoices:read',      'View invoices and bills'),
    ('invoices:write',     'Create and update invoices'),
    ('reports:read',       'View financial reports'),
    ('users:read',         'View organization members'),
    ('users:write',        'Invite and update members'),
    ('users:delete',       'Remove members'),
    ('roles:read',         'View roles and permissions'),
    ('roles:write',        'Create and manage custom roles');

-- ─── Roles ────────────────────────────────────────────────────────────────────

CREATE TABLE roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- NULL = system role (available to every org); non-NULL = org-custom role
    org_id      UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    is_system   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partial unique indexes because standard UNIQUE treats NULL as non-equal.
CREATE UNIQUE INDEX idx_roles_system_name ON roles(name)          WHERE org_id IS NULL;
CREATE UNIQUE INDEX idx_roles_org_name    ON roles(org_id, name)  WHERE org_id IS NOT NULL;

-- System roles — fixed UUIDs referenced by the application constants.
INSERT INTO roles (id, name, is_system) VALUES
    ('00000000-0000-0000-0000-000000000001', 'owner',      TRUE),
    ('00000000-0000-0000-0000-000000000002', 'admin',      TRUE),
    ('00000000-0000-0000-0000-000000000003', 'accountant', TRUE),
    ('00000000-0000-0000-0000-000000000004', 'viewer',     TRUE);

-- ─── Role → Permission assignments ───────────────────────────────────────────

CREATE TABLE role_permissions (
    role_id       UUID NOT NULL REFERENCES roles(id)       ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- viewer: read-only access
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000004', id FROM permissions
WHERE name IN (
    'accounts:read', 'transactions:read', 'contacts:read',
    'invoices:read', 'reports:read'
);

-- accountant: viewer + write access (no delete, no user/role management)
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000003', id FROM permissions
WHERE name IN (
    'accounts:read',     'accounts:write',
    'transactions:read', 'transactions:write',
    'contacts:read',     'contacts:write',
    'invoices:read',     'invoices:write',
    'reports:read'
);

-- admin: accountant + delete accounts + user management + roles:read
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000002', id FROM permissions
WHERE name IN (
    'accounts:read',     'accounts:write',     'accounts:delete',
    'transactions:read', 'transactions:write',
    'contacts:read',     'contacts:write',
    'invoices:read',     'invoices:write',
    'reports:read',
    'users:read',        'users:write',
    'roles:read'
);

-- owner: all permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000001', id FROM permissions;

-- ─── Migrate users.role TEXT → users.role_id UUID ────────────────────────────

ALTER TABLE users ADD COLUMN role_id UUID REFERENCES roles(id);

UPDATE users SET role_id = '00000000-0000-0000-0000-000000000001' WHERE role = 'owner';
UPDATE users SET role_id = '00000000-0000-0000-0000-000000000002' WHERE role = 'admin';
UPDATE users SET role_id = '00000000-0000-0000-0000-000000000003' WHERE role = 'accountant';
UPDATE users SET role_id = '00000000-0000-0000-0000-000000000004' WHERE role = 'viewer';
-- Any unmapped users fall back to viewer
UPDATE users SET role_id = '00000000-0000-0000-0000-000000000004' WHERE role_id IS NULL;

ALTER TABLE users ALTER COLUMN role_id SET NOT NULL;
ALTER TABLE users DROP COLUMN role;

CREATE INDEX idx_users_org_role ON users(organization_id, role_id);
