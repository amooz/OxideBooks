# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build
cargo build --release

# Check (fast, no codegen)
cargo check
cargo check --package oxidebooks-core
cargo check --package oxidebooks-db
cargo check --package oxidebooks-api

# Tests
cargo test
cargo test --package oxidebooks-core          # unit tests only (no DB required)
cargo test <test_name>                         # run a single test by name

# Linting & formatting
cargo clippy -- -D warnings
cargo fmt
cargo fmt --check

# Run the server (requires a running Postgres)
DATABASE_URL=postgresql://oxidebooks:password@localhost:5432/oxidebooks \
  OXIDEBOOKS__AUTH__JWT_SECRET=dev-secret \
  cargo run --package oxidebooks-api

# Docker (self-hosted stack)
cp .env.example .env          # then edit .env
docker compose up --build     # starts Postgres + API
docker compose --profile dev up   # also starts Adminer on :8080
```

## Architecture

This is a **Cargo workspace** with three crates:

| Crate | Role |
|---|---|
| `crates/core` | Pure domain models, validation, error types, money utilities. No I/O. |
| `crates/db` | PostgreSQL persistence via SQLx. Migrations, repository structs. |
| `crates/api` | Axum HTTP server. Config loading, JWT middleware, route/handler wiring. |

### Request lifecycle

```
HTTP request
  → Axum router (routes/mod.rs)
  → require_auth middleware (middleware/auth.rs)   [validates JWT, injects Claims]
  → handler (handlers/*.rs)
  → repo (crates/db/src/repos/*.rs)
  → PostgreSQL
```

`AppState` (cloned cheaply via `Arc`) carries the `PgPool` and loaded `Settings` into every handler via Axum's `State` extractor.

### Data / domain model

All monetary values are stored and computed as **integer minor units** (`i64`, type alias `MinorUnits` in `core/src/money.rs`). $100.50 → `10050`. Never use floats for money.

IDs are **UUID v4**, represented as `String` in domain models (serializes as hyphenated string in JSON) and as native `uuid::Uuid` in the PostgreSQL row structs.

Timestamps are `time::OffsetDateTime` (always UTC). Dates (journal entry date, invoice date) are `time::Date`.

Double-entry invariant: every `JournalEntry` must have `Σ debits == Σ credits` and at least two lines. Validation lives on `CreateJournalEntry::validate()` and is called inside `TransactionRepo::create` before any DB writes.

### Authentication & tenancy

The system is multi-tenant. Every data table has an `organization_id` column. All queries filter by `organization_id` extracted from the JWT `org` claim — handlers never accept an `org_id` from the request body.

Roles (ascending privilege): `viewer` → `accountant` → `admin` → `owner`. Helpers on `Claims`: `is_at_least_accountant()`, `is_admin()`.

Passwords are hashed with **Argon2** (`argon2` crate). Tokens are **HS256 JWT** (`jsonwebtoken` crate).

### Configuration

Settings are loaded by `crates/api/src/config.rs` via the `config` crate with this priority (highest wins):

1. Environment variables prefixed `OXIDEBOOKS__` with `__` as separator (e.g. `OXIDEBOOKS__DATABASE__URL`)
2. `config.toml` in the working directory
3. Compiled-in defaults

### Database & migrations

Migrations live in `crates/db/migrations/` and are embedded at compile time via `sqlx::migrate!("./migrations")`. They run automatically on startup before the server begins accepting requests.

The DB layer uses `sqlx::query_as` (runtime query building, not the compile-time `query!` macro) to avoid needing a live database at compile time. Each repo defines private `*Row` structs with `#[derive(sqlx::FromRow)]` and converts them to domain types.

### Error propagation

`CoreError` (domain validation) → `DbError` (database layer) → `ApiError` (HTTP layer, implements `IntoResponse`). The `ApiError::Db` variant pattern-matches on `DbError::is_not_found()` and `DbError::is_conflict()` to produce the right HTTP status codes. Internal errors log details but return a generic message to the client.

### Adding a new resource

1. Add model structs to `crates/core/src/models/`
2. Add a migration in `crates/db/migrations/` (filename: `NNNN_description.sql`)
3. Add a repo in `crates/db/src/repos/` and re-export from `repos/mod.rs`
4. Add handlers in `crates/api/src/handlers/`
5. Wire routes in `crates/api/src/routes/mod.rs`
