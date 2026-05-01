# OxideBooks

A self-hosted, multi-tenant accounting API built in Rust. Designed for businesses that need a reliable double-entry bookkeeping backend with enterprise authentication (OIDC/SAML SSO, SCIM provisioning) and fine-grained role-based access control.

## Features

- **Double-entry bookkeeping** — journal entries, chart of accounts, trial balance
- **Invoices & bills** — receivables and payables with line-item tax support
- **Contact management** — customers, vendors, or both
- **Multi-tenancy** — every resource is scoped to an organization; no data leakage between tenants
- **RBAC** — four system roles (owner, admin, accountant, viewer) plus org-custom roles with per-permission grants
- **Local authentication** — email/password with Argon2 hashing and HS256 JWT
- **SSO** — OIDC (authorization code + PKCE) and SAML 2.0 (SP-initiated redirect binding)
- **SCIM 2.0** — automated user provisioning/deprovisioning from identity providers
- **PostgreSQL** — Migrations embedded and run automatically at startup

## Quick Start

### Docker (recommended)

```bash
cp .env.example .env           # set DB_PASSWORD and JWT_SECRET
docker compose up --build      # starts PostgreSQL + API on :3000
docker compose --profile dev up  # also starts Adminer on :8080
```

### Local development

```bash
# Requires a running PostgreSQL instance
createdb oxidebooks

DATABASE_URL=postgresql://localhost/oxidebooks \
OXIDEBOOKS__AUTH__JWT_SECRET=dev-secret-min-32-chars \
  cargo run --package oxidebooks-api
```

See [docs/development.md](docs/development.md) for full setup instructions.

## Documentation

| Document | Contents |
|---|---|
| [docs/architecture.md](docs/architecture.md) | Crate layout, data model, request lifecycle, key design decisions |
| [docs/api.md](docs/api.md) | Complete REST API reference |
| [docs/auth.md](docs/auth.md) | Authentication flows: local, OIDC, SAML, SCIM |
| [docs/rbac.md](docs/rbac.md) | Roles, permissions, and access control model |
| [docs/development.md](docs/development.md) | Running locally, tests, configuration reference |

## License

AGPL-3.0
