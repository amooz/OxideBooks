# Roles & Permissions

OxideBooks uses a flat, explicit permission model. Every action in the API requires a specific permission string. Permissions are assigned to roles; roles are assigned to users.

---

## Permissions

There are 15 named permissions, all with the format `{resource}:{action}`:

| Permission | Description |
|---|---|
| `accounts:read` | View chart of accounts |
| `accounts:write` | Create and update accounts |
| `accounts:delete` | Delete accounts |
| `contacts:read` | View contacts |
| `contacts:write` | Create and update contacts |
| `invoices:read` | View invoices and bills |
| `invoices:write` | Create invoices and bills |
| `transactions:read` | View journal entries |
| `transactions:write` | Create journal entries |
| `reports:read` | View financial reports |
| `users:read` | View users and identity providers |
| `users:write` | Create users, manage SSO providers, SCIM tokens |
| `users:delete` | Delete users |
| `roles:read` | View roles and their permissions |
| `roles:write` | Create roles, assign/remove permissions |

---

## System Roles

Four system roles are seeded with fixed UUIDs at startup. They are global (not scoped to any organization) and their permission sets cannot be modified.

| Role | UUID suffix | Permissions |
|---|---|---|
| `viewer` | `...0004` | accounts:read, contacts:read, invoices:read, transactions:read, reports:read |
| `accountant` | `...0003` | viewer + accounts:write, contacts:write, invoices:write, transactions:write |
| `admin` | `...0002` | accountant + accounts:delete, users:read, users:write, roles:read |
| `owner` | `...0001` | All 15 permissions |

Full UUIDs:
- `00000000-0000-0000-0000-000000000001` — owner
- `00000000-0000-0000-0000-000000000002` — admin
- `00000000-0000-0000-0000-000000000003` — accountant
- `00000000-0000-0000-0000-000000000004` — viewer

---

## Custom Roles

Organizations can create custom roles and assign any subset of the 15 permissions.

**Create a role:**
```http
POST /api/v1/roles
Authorization: Bearer <owner-or-admin-jwt>

{ "name": "billing-manager" }
```

**Assign permissions:**
```http
POST /api/v1/roles/{role_id}/permissions
Authorization: Bearer <owner-or-admin-jwt>

{ "permission": "invoices:write" }
```

Assignment is idempotent — posting the same permission twice has no effect.

**Remove a permission:**
```http
DELETE /api/v1/roles/{role_id}/permissions/invoices:write
```

Custom roles are scoped to the organization that created them. System roles (`org_id = null`) are visible to all organizations but cannot be modified.

---

## How Permissions Are Checked

Permissions are embedded in the JWT at login time:

```json
{
  "sub": "user-uuid",
  "org": "org-uuid",
  "role": "accountant",
  "permissions": ["accounts:read", "accounts:write", "transactions:read", "..."],
  "exp": 1700000000
}
```

Each handler calls `claims.has("resource:action")` before doing any work. This is an exact string match — no hierarchy or wildcards.

Because permissions are embedded in the JWT, changes to a user's role or a role's permissions take effect the next time the user logs in (when a new token is issued).

---

## Example: First-Time Setup

After registering, the first user has the `owner` role with all permissions. To invite a bookkeeper:

1. Create a user account for them (they register or are provisioned via SCIM).
2. Currently, the user creation via API requires a `users:write` permission — handled by an owner. The user will initially get whatever role is assigned during creation.
3. To restrict their access to accountant-level, ensure they are assigned to the `accountant` role.

---

## Listing Available Roles and Permissions

```http
GET /api/v1/permissions     # all 15 permission objects
GET /api/v1/roles           # system roles + org-custom roles with their permissions
```

Both endpoints require `roles:read` permission.
