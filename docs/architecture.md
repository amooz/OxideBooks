# Architecture

## Crate Layout

OxideBooks is a Cargo workspace split into three crates with strict layering — each layer may only depend on layers below it.

```
crates/
  core/   Pure domain: models, validation, money utilities. No I/O, no dependencies on db or api.
  db/     PostgreSQL persistence via SQLx. Imports core. No HTTP.
  api/    Axum HTTP server. Imports core and db. Wires config, middleware, routes, handlers.
```

### `oxidebooks-core`

Contains all business logic that is independent of storage or transport:

- **Models** — Rust structs that represent the domain, each in `src/models/<name>.rs`
- **Validation** — `CreateJournalEntry::validate()` enforces double-entry rules; `CreateInvoice::validate()` enforces positive quantities. Both return `CoreError` on failure.
- **Money** — `type MinorUnits = i64`. All monetary amounts are integer minor units (e.g. $1.00 = `100`, ¥100 = `100`). `format_amount(amount, currency)` handles display, including zero-decimal currencies (JPY, KRW) and three-decimal currencies (KWD, BHD, OMR).
- **`CoreError`** — typed validation errors propagated up through `DbError` and `ApiError`.

### `oxidebooks-db`

Persistence only — no HTTP types:

- **Repositories** — one struct per resource (`AccountRepo`, `TransactionRepo`, etc.), all methods are `async fn` that take `&PgPool`.
- **Row types** — private `*Row` structs with `#[derive(sqlx::FromRow)]`, converted to domain types with `TryFrom`/`From`.
- **`DbError`** — wraps `sqlx::Error` and `CoreError`; has `is_not_found()` and `is_conflict()` helpers for HTTP status mapping.
- **`MIGRATOR`** — `sqlx::Migrator` embedded from `migrations/` at compile time; run automatically on startup.
- **`pub static MIGRATOR`** — exported so test harnesses (`#[sqlx::test(migrator = ...)]`) can spin up a fresh schema per test.

All queries use `sqlx::query_as` (runtime, not the compile-time `query!` macro) so the build never requires a live database.

### `oxidebooks-api`

HTTP server:

- **`AppState`** — `{db: PgPool, config: Arc<Settings>}`, cheaply cloned into every handler via Axum's `State` extractor.
- **`config.rs`** — `Settings::load()` reads from environment variables (`OXIDEBOOKS__*`) > `config.toml` > compiled defaults.
- **`middleware/auth.rs`** — `require_auth` Axum layer validates HS256 JWT, injects `Claims` into request extensions.
- **`error.rs`** — `ApiError` implements `IntoResponse`; maps `DbError` to appropriate HTTP status codes.
- **`routes/mod.rs`** — wires all handlers; protected routes apply `require_auth` as a layer.
- **`handlers/`** — one file per resource; each handler extracts `Claims`, checks a permission with `claims.has("resource:action")`, delegates to a repo.

The API crate is structured as a library (`src/lib.rs`) with a thin binary (`src/main.rs`), allowing integration tests in `tests/` to `use oxidebooks_api::...`.

---

## Request Lifecycle

```
HTTP request
  └─ Axum router (routes/mod.rs)
       └─ require_auth middleware      validates JWT, injects Claims
            └─ handler (handlers/*.rs)
                 ├─ claims.has("resource:action")   → 403 if denied
                 ├─ repo call (db/src/repos/*.rs)
                 │    ├─ domain validation (core)   → 422 if invalid
                 │    └─ SQL via sqlx               → DbError on failure
                 └─ ApiError::from(DbError)         → correct HTTP status
```

---

## Data Model

### Multi-Tenancy

Every business table (`accounts`, `journal_entries`, `journal_lines`, `invoices`, `invoice_lines`, `contacts`) has an `organization_id UUID` column with a foreign key to `organizations`. All repository queries filter by this column using the `org` claim from the JWT — it is never accepted from the request body.

### Money

All monetary values are `BIGINT` in the database and `i64` (`MinorUnits`) in Rust. The unit is always the smallest denomination of the currency (cents for USD/EUR/GBP, pence for GBP, etc.). Floating-point arithmetic is never used for money.

Quantity in invoice lines is also integer ×100 (so 1.5 units = `150`). Tax rate is ×100 (10% = `1000`).

### Double-Entry Invariant

`journal_lines` has a check constraint: `NOT (debit > 0 AND credit > 0)`. The application layer enforces additionally:
- At least two lines per entry
- Sum of debits == sum of credits

Validation happens in `CreateJournalEntry::validate()` (core layer) before any database write.

### IDs and Timestamps

- **IDs**: UUID v4, stored as native `uuid` in PostgreSQL, serialized as hyphenated strings in JSON.
- **Timestamps**: `timestamptz` in PostgreSQL, `time::OffsetDateTime` in Rust, RFC 3339 in JSON. Always UTC.
- **Dates**: `date` in PostgreSQL, `time::Date` in Rust, `"YYYY-MM-DD"` in JSON. Used for journal entry date and invoice dates (no time component).

---

## Database Migrations

Migrations live in `crates/db/migrations/` and are applied in order at startup:

| File | Purpose |
|---|---|
| `0001_initial.sql` | Core schema: organizations, users, accounts, journal_entries, journal_lines, contacts, invoices, invoice_lines |
| `0002_rbac.sql` | RBAC: permissions table, roles table, role_permissions join table; migrates users.role TEXT → users.role_id UUID |
| `0003_identity.sql` | SSO & provisioning: identity_providers, oidc_states, scim_tokens; extends users with auth_method, identity_provider_id, external_id |

---

## Error Propagation

```
CoreError  (domain validation: UnbalancedEntry, NegativeAmount, …)
    ↓  wrapped by
DbError    (NotFound, Conflict, Internal, Sqlx)
    ↓  mapped by
ApiError   (NotFound→404, Conflict→409, Validation→422, Internal→500)
    ↓  IntoResponse
JSON: {"error": {"code": "…", "message": "…"}}
```

Internal errors log the full cause but return a redacted message to the client.

---

## Key Design Decisions

**Integer money** — avoids floating-point rounding errors in financial calculations.

**Runtime query building** — `sqlx::query_as` instead of `query!` so `cargo build` never requires a live database. The tradeoff is that SQL type errors are caught at runtime, not compile time.

**Permissions in JWT** — permissions are resolved at login time and embedded in the JWT, so handlers can check `claims.has("accounts:write")` without a database lookup per request. The tradeoff is that permission changes take effect at next login.

**Domain validation first** — `validate()` on `Create*` types runs before any SQL, keeping business rules in `core` (testable without a database).

**Fixed system role UUIDs** — `ROLE_OWNER_ID = "00000000-0000-0000-0000-000000000001"` etc. are constants in code and stable across deployments, so queries can reference them without a lookup.

**No compile-time SQL** — intentional; avoids requiring `DATABASE_URL` in CI unless specifically running database integration tests.
