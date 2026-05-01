# Development Guide

## Prerequisites

- Rust (stable, 1.80+) — install via [rustup](https://rustup.rs)
- PostgreSQL 14+ running locally, or Docker

## Local Setup

```bash
# Clone and enter the repo
git clone https://github.com/amooz/OxideBooks
cd OxideBooks

# Create the database
createdb oxidebooks

# Copy and edit the example config (optional — env vars work too)
cp config.toml.example config.toml

# Run the server
OXIDEBOOKS__DATABASE__URL=postgresql://localhost/oxidebooks \
OXIDEBOOKS__AUTH__JWT_SECRET="dev-secret-must-be-at-least-32-chars" \
  cargo run --package oxidebooks-api
```

The server starts on `http://localhost:3000`. Migrations run automatically on startup.

## Docker

```bash
cp .env.example .env       # edit DB_PASSWORD and JWT_SECRET

docker compose up --build  # PostgreSQL + API

# Optional: Adminer web UI at http://localhost:8080
docker compose --profile dev up
```

## Configuration

Settings are loaded with this priority (highest wins):

1. Environment variables prefixed `OXIDEBOOKS__` with `__` as separator
2. `config.toml` in the working directory
3. Compiled-in defaults

### All Settings

| Environment variable | `config.toml` key | Default | Description |
|---|---|---|---|
| `OXIDEBOOKS__SERVER__HOST` | `server.host` | `0.0.0.0` | Bind address |
| `OXIDEBOOKS__SERVER__PORT` | `server.port` | `3000` | Listen port |
| `OXIDEBOOKS__DATABASE__URL` | `database.url` | (sqlite dev default) | PostgreSQL connection URL |
| `OXIDEBOOKS__AUTH__JWT_SECRET` | `auth.jwt_secret` | `change-me` | HS256 signing key (min 32 chars) |
| `OXIDEBOOKS__AUTH__TOKEN_EXPIRY_HOURS` | `auth.token_expiry_hours` | `24` | JWT lifetime |
| `OXIDEBOOKS__AUTH__REFRESH_EXPIRY_DAYS` | `auth.refresh_expiry_days` | `30` | (reserved) |
| `OXIDEBOOKS__APP__REGISTRATION_OPEN` | `app.registration_open` | `true` | Allow new org registration |
| `OXIDEBOOKS__APP__DEFAULT_CURRENCY` | `app.default_currency` | `USD` | Default invoice currency |
| `OXIDEBOOKS__APP__BASE_URL` | `app.base_url` | `http://localhost:3000` | Public API URL (for OAuth2 redirects, SAML ACS) |

`RUST_LOG` controls tracing output (e.g. `RUST_LOG=info,oxidebooks=debug`).

## Commands

```bash
# Build
cargo build
cargo build --release

# Type check (fast, no codegen)
cargo check
cargo check --package oxidebooks-core
cargo check --package oxidebooks-db
cargo check --package oxidebooks-api

# Linting — warnings are errors
cargo clippy -- -D warnings

# Format check / apply
cargo fmt --check
cargo fmt

# Run all tests (unit + integration; requires PostgreSQL)
cargo test

# Unit tests only (no database required)
cargo test --package oxidebooks-core

# Single test by name
cargo test test_name
```

## Tests

### Unit tests (`oxidebooks-core`)

All domain model tests live alongside their code in `crates/core/src/models/*.rs`. Run without a database:

```bash
cargo test --package oxidebooks-core
```

### DB integration tests (`oxidebooks-db`)

Located in `crates/db/tests/`. Each test uses `#[sqlx::test(migrator = "oxidebooks_db::MIGRATOR")]` which spins up an isolated PostgreSQL schema per test:

```bash
DATABASE_URL=postgresql://localhost/oxidebooks cargo test --package oxidebooks-db
```

### Handler function tests (`oxidebooks-api`)

Located in `crates/api/tests/`. Build the full Axum router backed by a real test database and send HTTP requests via `tower::ServiceExt::oneshot`:

```bash
DATABASE_URL=postgresql://localhost/oxidebooks cargo test --package oxidebooks-api
```

The `handler_helpers.rs` file provides:
- `build_app(pool)` — builds the full router
- `mint_test_token(user_id, org_id, role, permissions)` — creates a signed test JWT
- `send(app, request)` — fires one request through the router
- `json_body(response)` — parses the response body as JSON

## Workspace Structure

```
OxideBooks/
├── Cargo.toml                  # workspace manifest + shared dependencies
├── Dockerfile                  # multi-stage: builder (rust:1.82) + runtime (debian slim)
├── docker-compose.yml          # PostgreSQL + API + optional Adminer
├── config.toml.example         # example config file
├── docs/                       # this documentation
│   ├── architecture.md
│   ├── api.md
│   ├── auth.md
│   ├── rbac.md
│   └── development.md
└── crates/
    ├── core/
    │   └── src/
    │       ├── lib.rs
    │       ├── error.rs        # CoreError enum
    │       ├── money.rs        # MinorUnits type, format_amount()
    │       └── models/
    │           ├── mod.rs
    │           ├── account.rs
    │           ├── contact.rs
    │           ├── identity.rs # SSO/SCIM models
    │           ├── invoice.rs
    │           ├── organization.rs
    │           ├── reports.rs
    │           ├── role.rs
    │           └── transaction.rs
    ├── db/
    │   ├── migrations/
    │   │   ├── 0001_initial.sql
    │   │   ├── 0002_rbac.sql
    │   │   └── 0003_identity.sql
    │   └── src/
    │       ├── lib.rs          # exports: PgPool, MIGRATOR, all repos
    │       ├── error.rs        # DbError enum
    │       └── repos/
    │           ├── mod.rs
    │           ├── accounts.rs
    │           ├── contacts.rs
    │           ├── identity.rs  # IdentityProviderRepo, ScimTokenRepo
    │           ├── invoices.rs
    │           ├── organizations.rs
    │           ├── permissions.rs
    │           ├── reports.rs
    │           ├── roles.rs
    │           ├── transactions.rs
    │           └── users.rs
    └── api/
        ├── src/
        │   ├── lib.rs           # re-exports for integration tests
        │   ├── main.rs          # binary entry point
        │   ├── config.rs        # Settings struct
        │   ├── error.rs         # ApiError → IntoResponse
        │   ├── state.rs         # AppState
        │   ├── middleware/
        │   │   └── auth.rs      # require_auth, Claims
        │   ├── routes/
        │   │   └── mod.rs       # build(state) → Router
        │   └── handlers/
        │       ├── mod.rs
        │       ├── accounts.rs
        │       ├── auth.rs      # register, login
        │       ├── auth_sso.rs  # OIDC + SAML flows
        │       ├── contacts.rs
        │       ├── identity.rs  # IdP + SCIM token management
        │       ├── invoices.rs
        │       ├── reports.rs
        │       ├── roles.rs
        │       ├── scim.rs      # SCIM 2.0 user endpoints
        │       └── transactions.rs
        └── tests/
            ├── handler_helpers.rs
            ├── accounts_handler_test.rs
            └── roles_handler_test.rs
```

## Adding a New Resource

1. Add model structs to `crates/core/src/models/<name>.rs` and re-export from `models/mod.rs`
2. Write a migration in `crates/db/migrations/<NNNN>_<name>.sql`
3. Add a repo in `crates/db/src/repos/<name>.rs` and re-export from `repos/mod.rs`
4. Add handlers in `crates/api/src/handlers/<name>.rs`
5. Register handlers in `crates/api/src/routes/mod.rs`
6. Write unit tests in the model file and integration tests in `crates/db/tests/` and `crates/api/tests/`

## Common Issues

**`cargo check` fails with missing database URL** — this shouldn't happen (the project uses `sqlx::query_as`, not compile-time `query!`). If it does, check that no `query!` macros crept in.

**Tests fail with "could not connect to server"** — ensure PostgreSQL is running and `DATABASE_URL` is set.

**JWT "invalid signature" errors** — ensure `OXIDEBOOKS__AUTH__JWT_SECRET` matches between token generation and validation (both server instances must use the same secret).

**OIDC callback "state not found"** — states expire after 15 minutes. Ensure the authorize and callback happen within that window, and that you're not reusing a state.
