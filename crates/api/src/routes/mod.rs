use std::{sync::Arc, time::Duration};

use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    handlers::{
        accounts, audit, auth, auth_sso, contacts, exchange_rates, expenses, health, identity,
        invoices, organizations, payments, recurring, reports, roles, scim, tax_rates,
        transactions, users,
    },
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let cors = build_cors(&state.config.app.allowed_origins);

    // Rate limiters — keyed on client IP, leaky-bucket via governor.
    // login: 10 req/min
    let login_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(6))
            .burst_size(10)
            .finish()
            .expect("rate limiter config"),
    );
    // register: 5 req/min
    let register_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(12))
            .burst_size(5)
            .finish()
            .expect("rate limiter config"),
    );
    // SSO initiation + callbacks: 20 req/min
    let sso_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(3))
            .burst_size(20)
            .finish()
            .expect("rate limiter config"),
    );

    // Rate limiter for provider discovery (20 req/min — same as SSO)
    let discovery_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(3))
            .burst_size(20)
            .finish()
            .expect("rate limiter config"),
    );

    // Public auth routes (no JWT required)
    let public = Router::new()
        .route(
            "/auth/register",
            post(auth::register).layer(GovernorLayer {
                config: register_rl,
            }),
        )
        .route(
            "/auth/login",
            post(auth::login).layer(GovernorLayer { config: login_rl }),
        )
        .route(
            "/auth/providers",
            get(auth_sso::list_providers).layer(GovernorLayer {
                config: discovery_rl,
            }),
        )
        // SSO initiation & callbacks (public — redirect-based flows)
        .route(
            "/auth/oidc/{provider_id}",
            get(auth_sso::oidc_initiate).layer(GovernorLayer {
                config: sso_rl.clone(),
            }),
        )
        .route(
            "/auth/oidc/{provider_id}/callback",
            get(auth_sso::oidc_callback).layer(GovernorLayer {
                config: sso_rl.clone(),
            }),
        )
        .route(
            "/auth/saml/{provider_id}",
            get(auth_sso::saml_initiate).layer(GovernorLayer {
                config: sso_rl.clone(),
            }),
        )
        .route(
            "/auth/saml/{provider_id}/callback",
            post(auth_sso::saml_callback).layer(GovernorLayer {
                config: sso_rl.clone(),
            }),
        )
        .route(
            "/auth/saml/{provider_id}/metadata",
            get(auth_sso::saml_sp_metadata),
        );

    let protected = Router::new()
        // Chart of Accounts
        .route(
            "/accounts",
            get(accounts::list_accounts).post(accounts::create_account),
        )
        .route(
            "/accounts/:id",
            get(accounts::get_account)
                .patch(accounts::update_account)
                .delete(accounts::delete_account),
        )
        // Auth (authenticated)
        .route("/auth/password", post(auth::change_password))
        // Journal entries
        .route(
            "/transactions",
            get(transactions::list_transactions).post(transactions::create_transaction),
        )
        .route(
            "/transactions/:id",
            get(transactions::get_transaction).patch(transactions::void_transaction),
        )
        // Contacts
        .route(
            "/contacts",
            get(contacts::list_contacts).post(contacts::create_contact),
        )
        .route(
            "/contacts/:id",
            get(contacts::get_contact)
                .patch(contacts::update_contact)
                .delete(contacts::delete_contact),
        )
        // Invoices & bills
        .route(
            "/invoices",
            get(invoices::list_invoices).post(invoices::create_invoice),
        )
        .route(
            "/invoices/:id",
            get(invoices::get_invoice).patch(invoices::update_invoice),
        )
        .route(
            "/invoices/:id/payments",
            post(payments::create_payment).get(payments::list_payments),
        )
        // Organization settings
        .route(
            "/organizations/me",
            get(organizations::get_org).patch(organizations::update_org),
        )
        // User management
        .route("/users", get(users::list_users).post(users::invite_user))
        .route(
            "/users/:id",
            get(users::get_user)
                .patch(users::update_user)
                .delete(users::delete_user),
        )
        // Reports
        .route("/reports/trial-balance", get(reports::trial_balance))
        .route("/reports/profit-loss", get(reports::profit_loss))
        .route("/reports/balance-sheet", get(reports::balance_sheet))
        .route("/reports/aging", get(reports::aging))
        .route("/reports/tax-summary", get(reports::tax_summary))
        // Dashboard
        .route("/dashboard", get(reports::dashboard))
        // Audit log
        .route("/audit-log", get(audit::list_audit_log))
        // Tax rates
        .route(
            "/tax-rates",
            get(tax_rates::list_tax_rates).post(tax_rates::create_tax_rate),
        )
        .route(
            "/tax-rates/:id",
            get(tax_rates::get_tax_rate)
                .patch(tax_rates::update_tax_rate)
                .delete(tax_rates::delete_tax_rate),
        )
        // Expenses
        .route(
            "/expenses",
            get(expenses::list_expenses).post(expenses::create_expense),
        )
        .route(
            "/expenses/:id",
            get(expenses::get_expense).patch(expenses::update_expense),
        )
        .route("/expenses/:id/submit", post(expenses::submit_expense))
        .route("/expenses/:id/approve", post(expenses::approve_expense))
        .route("/expenses/:id/reject", post(expenses::reject_expense))
        .route("/expenses/:id/reimburse", post(expenses::reimburse_expense))
        // Recurring schedules
        .route(
            "/recurring-schedules",
            get(recurring::list_schedules).post(recurring::create_schedule),
        )
        .route(
            "/recurring-schedules/:id",
            get(recurring::get_schedule)
                .patch(recurring::update_schedule)
                .delete(recurring::delete_schedule),
        )
        // RBAC: permissions & roles
        .route("/permissions", get(roles::list_permissions))
        .route("/roles", get(roles::list_roles).post(roles::create_role))
        .route("/roles/:id/permissions", post(roles::assign_permission))
        .route(
            "/roles/:role_id/permissions/:permission",
            delete(roles::remove_permission),
        )
        // Identity providers (admin-managed)
        .route("/identity-providers", get(identity::list_providers))
        .route(
            "/identity-providers/oidc",
            post(identity::create_oidc_provider),
        )
        .route(
            "/identity-providers/saml",
            post(identity::create_saml_provider),
        )
        .route("/identity-providers/:id", delete(identity::delete_provider))
        // SCIM token management (admin-managed)
        .route(
            "/scim/tokens",
            get(identity::list_scim_tokens).post(identity::create_scim_token),
        )
        .route("/scim/tokens/:id", delete(identity::revoke_scim_token))
        // Exchange rates
        .route("/exchange-rates", get(exchange_rates::get_rate))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // SCIM 2.0 endpoints (separate bearer-token auth, not JWT)
    let scim_routes = Router::new()
        .route(
            "/scim/v2/ServiceProviderConfig",
            get(scim::service_provider_config),
        )
        .route(
            "/scim/v2/Users",
            get(scim::list_users).post(scim::create_user),
        )
        .route(
            "/scim/v2/Users/:id",
            get(scim::get_user)
                .patch(scim::patch_user)
                .delete(scim::delete_user),
        );

    Router::new()
        .nest("/api/v1", public.merge(protected))
        .merge(scim_routes)
        .route("/health", get(health::readiness))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins == ["*"] {
        return CorsLayer::permissive();
    }
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
}
