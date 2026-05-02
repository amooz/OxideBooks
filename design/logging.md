# Logging Philosophy

OxideBooks uses the [`tracing`](https://docs.rs/tracing) ecosystem for structured, leveled logging. Logs are the primary operational visibility tool for a running server — they must be useful to the person on call at 2 am, parseable by a log aggregator, and free of noise that trains operators to ignore them.

---

## The Golden Rule

> **A log line is a contract with the operator. Every line that appears must be worth the attention it demands.**

A log that appears on every request teaches humans and machines to ignore it. A log that appears when something surprising happens earns trust. Prefer silence over noise.

---

## Log Levels

### `error!` — Something broke and needs human attention

The system encountered a condition it cannot recover from on its own. An operator should investigate within minutes.

An `error!` event means:
- A database query failed unexpectedly
- An external service returned an unrecoverable error
- A security-relevant invariant was violated
- Data corruption or consistency loss may have occurred

An `error!` does **not** mean:
- A client sent a bad request (that's the client's fault — log at `debug!` if at all)
- A resource was not found (normal application flow)
- A JWT expired (expected user behavior)

```rust
// ✅ Good — unexpected DB failure, operator needs to know
tracing::error!(
    error = %e,
    user_id = %user_id,
    "🔴 failed to write journal entry — database unavailable"
);

// ❌ Bad — this is normal client behavior, not a server error
tracing::error!("user not found: {id}");
```

---

### `warn!` — Unexpected, but the system handled it

The system did something it didn't expect to do, recovered gracefully, but an operator should be aware the condition exists. A steady stream of `warn!` events indicates a problem worth investigating before it becomes an `error!`.

A `warn!` event means:
- A retry succeeded after an initial failure
- A configuration value is missing and a fallback was used
- A deprecated API path was called
- A SCIM token was used from an unusual IP or at an unusual rate
- A migration was skipped because it was already applied

```rust
// ✅ Good — degraded behavior worth noting
tracing::warn!(
    provider_id = %provider_id,
    "⚠️ OIDC discovery metadata cached — refresh failed, using stale copy"
);

// ❌ Bad — this is routine, not degraded behavior
tracing::warn!("no results found for query");
```

---

### `info!` — Normal, significant operational events

Milestones in the server lifecycle and per-request events that are worth recording for audit or operational awareness. Not every request — only significant transitions.

An `info!` event means:
- Server started / stopped
- Database connection established; migrations applied
- A user authenticated (login, SSO callback)
- An organization was created
- A SCIM provisioning operation completed
- A new identity provider was configured

```rust
// ✅ Good — significant lifecycle event
tracing::info!(
    user_id = %user.id,
    org_id = %org_id,
    method = "oidc",
    "🔐 user authenticated via SSO"
);

// ❌ Bad — too granular for info, creates noise in prod
tracing::info!("fetching account list for org {org_id}");
```

---

### `debug!` — Diagnostic detail for troubleshooting

Enabled during development and targeted investigations. Not expected to be on in production except when diagnosing a specific incident. Every `debug!` line should answer a likely debugging question.

A `debug!` event means:
- "What path did the code take here?"
- "What value did we compute from this input?"
- "Which query did we run and what did it return?"

```rust
// ✅ Good — helps diagnose OIDC flow without PII in the token
tracing::debug!(
    provider_id = %provider_id,
    state_key = %state,
    "🔍 OIDC state stored, redirecting to IdP"
);

// ❌ Bad — logs the full token; even debug logs must not contain secrets
tracing::debug!("OIDC callback, code_verifier: {}", code_verifier);
```

---

### `trace!` — Extremely granular, per-step detail

Reserved for deep diagnosis of algorithmic or protocol-level behavior. Almost never enabled except in a local development environment with a specific subsystem under investigation.

```rust
// ✅ Good — step-by-step SAML XML parsing during local debugging
tracing::trace!(raw_xml = %xml_str, "🔬 parsing SAMLResponse assertion");
```

---

## Structured Fields vs Interpolation

Always prefer **structured fields** over string interpolation. Structured fields are machine-readable and searchable in log aggregators (Datadog, Loki, CloudWatch Logs Insights, etc.).

```rust
// ✅ Structured — org_id is a queryable field
tracing::info!(
    org_id = %claims.org,
    user_id = %claims.sub,
    invoice_id = %invoice.id,
    "📋 invoice created"
);

// ❌ Interpolated — org_id is buried in a string, unsearchable
tracing::info!("invoice created for org {} by user {}", claims.org, claims.sub);
```

### Field formatters

| Syntax | Meaning | Use for |
|---|---|---|
| `field = %value` | `Display` trait | UUIDs, strings, numbers, IPs |
| `field = ?value` | `Debug` trait | Enums, structs, complex types |
| `field = value` | Copy-able primitives | `bool`, `i64`, `u32` |

```rust
tracing::info!(
    org_id   = %org_id,        // Display — UUID string
    count    = rows.len(),     // primitive
    balanced = ?is_balanced,   // Debug on bool is fine but primitive is preferred
    "📊 trial balance computed"
);
```

---

## What Never Appears in Logs

Regardless of level, these values **must never appear** in any log line:

| Value | Reason |
|---|---|
| Passwords or password hashes | Credentials |
| JWT token strings | Bearer secrets |
| OIDC `code` or `code_verifier` | OAuth secrets |
| SCIM raw bearer tokens (`scim_...`) | Credentials |
| SAML assertions (full XML) | May contain identity claims |
| OIDC `client_secret` | IdP credentials |
| SAML `idp_certificate` PEM | Sensitive config |
| Full request/response bodies | May contain any of the above |

Log the **ID** of a sensitive object, never its value:

```rust
// ✅ Log the token ID, not the raw token
tracing::info!(token_id = %token.id, "🔑 SCIM token created");

// ❌ Never log the raw token
tracing::info!("SCIM token created: {}", raw_token);
```

---

## Message Text Style

Log messages (the string after the fields) follow these conventions:

- **Present tense, past participle for completions** — `"invoice created"`, not `"creating invoice"` or `"create invoice"`
- **Lowercase** — no sentence case, no trailing punctuation
- **Verb + noun** — `"user authenticated"`, `"migration applied"`, `"request rejected"`
- **Concise** — under 60 characters; details belong in structured fields
- **No redundancy with fields** — don't repeat field values in the message string

```rust
// ✅ Good
tracing::info!(user_id = %id, "user authenticated");

// ❌ Bad — message repeats the field
tracing::info!(user_id = %id, "user {} authenticated", id);

// ❌ Bad — too verbose, uses sentence case
tracing::info!("Successfully authenticated the user and issued a JWT token.");
```

---

## Emojis in Log Messages

Emojis are **permitted and encouraged** at `info!` and above in development and staging environments. They provide instant visual scanning in a terminal log stream where the eye needs to find signal among noise.

### Rules

1. **One emoji per message, at the start** — before the message text.
2. **Emoji choice is determined by the legend** — do not invent new emoji meanings.
3. **Emoji are for humans, not machines** — structured JSON logs for production aggregators should still parse fine (emoji are valid UTF-8), but don't rely on them for alerting.
4. **Never use emoji in `debug!` or `trace!`** — those levels are for diagnostic detail, not quick scanning.
5. **Never use emoji in error `message` strings used for alerting** — put them in the human-readable part only, not in field values that feed alert rules.

```rust
// ✅ Correct use
tracing::info!(org_id = %org_id, "🏢 organization created");
tracing::warn!(provider_id = %id, "⚠️ IdP response slow — above 2s threshold");
tracing::error!(error = %e, "🔴 database write failed");

// ❌ Wrong — emoji in debug level
tracing::debug!("🔍 checking PKCE verifier");  // no emoji at debug

// ❌ Wrong — multiple emoji
tracing::info!("🚀✅ server started");

// ❌ Wrong — emoji in a field value
tracing::error!(status = "🔴 failed", "write failed");
```

---

## Emoji Legend

This is the complete set of approved emoji for OxideBooks log messages. Any emoji not in this table must not be used.

| Emoji | Meaning | Levels | Example event |
|---|---|---|---|
| 🚀 | Server startup / boot | `info` | `"🚀 OxideBooks started"` |
| 🛑 | Server shutdown / stop | `info` | `"🛑 shutting down"` |
| 🏥 | Health check | `info` | `"🏥 health check passed"` |
| 🗄️ | Database operation | `info`, `warn` | `"🗄️ migrations applied"` |
| 🔴 | Hard error / failure | `error` | `"🔴 database write failed"` |
| ⚠️ | Warning / degraded | `warn` | `"⚠️ using stale OIDC metadata"` |
| ✅ | Successful completion | `info` | `"✅ journal entry posted"` |
| 🔐 | Authentication event | `info`, `warn` | `"🔐 user authenticated via SSO"` |
| 🔑 | Token / key lifecycle | `info` | `"🔑 SCIM token created"` |
| 🚫 | Authorization failure | `warn` | `"🚫 permission denied"` |
| 👤 | User lifecycle | `info` | `"👤 user provisioned via SCIM"` |
| 🏢 | Organization lifecycle | `info` | `"🏢 organization created"` |
| 📋 | Invoice / document | `info` | `"📋 invoice created"` |
| 💸 | Journal entry / payment | `info` | `"💸 journal entry posted"` |
| 📊 | Report generated | `info` | `"📊 trial balance computed"` |
| 🔄 | Sync / retry / refresh | `info`, `warn` | `"🔄 OIDC discovery refreshed"` |
| 🌐 | External HTTP call | `info`, `warn` | `"🌐 OIDC token exchange complete"` |
| 🔌 | Connection established | `info` | `"🔌 database connection pool ready"` |

---

## Span Context

Use `tracing::instrument` on handlers to automatically attach the request context to all log lines within that call tree. This links a chain of `debug!` and `info!` events to the same logical request without manually threading IDs.

```rust
#[tracing::instrument(
    skip(state, claims, payload),
    fields(
        org_id  = %claims.org,
        user_id = %claims.sub,
    )
)]
pub async fn create_transaction(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateJournalEntry>,
) -> ApiResult<impl IntoResponse> {
    // All tracing calls inside here automatically include org_id + user_id
    tracing::debug!(lines = payload.lines.len(), "validating journal entry");
    // ...
}
```

Fields declared in `fields(...)` appear on every event in the span. Use `skip(...)` for any argument that contains secrets or is too large to log (state, full request bodies).

---

## RUST_LOG Configuration

```bash
# Production — info for everything, debug for oxidebooks crate
RUST_LOG=info,oxidebooks=debug

# Troubleshoot OIDC flow
RUST_LOG=info,oxidebooks_api::handlers::auth_sso=debug

# Silence noisy crates during development
RUST_LOG=debug,sqlx=warn,hyper=warn,reqwest=warn

# Maximum verbosity (local only — never in prod)
RUST_LOG=trace
```

The `oxidebooks=debug` default in `main.rs` means OxideBooks crate code logs at `debug` while dependencies stay at `info`. This gives diagnostic detail without drowning in sqlx query logs.

---

## Level Decision Tree

```
Did the server fail to complete a request it should have been able to complete?
    Yes → Is the cause an unexpected internal error (DB down, panic, corruption)?
              Yes → error!
              No  → Was it the client's fault (bad input, auth failure, not found)?
                        Yes → debug! (or nothing)
                        No  → warn!
    No  → Is this a significant lifecycle event (login, create org, start server)?
              Yes → info!
              No  → Is this diagnostic detail useful when troubleshooting?
                        Yes → debug!
                        No  → Don't log it.
```
