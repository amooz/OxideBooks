use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};

use crate::{
    handlers::{
        accounts, auth, auth_sso, contacts, identity, invoices, organizations, reports, roles,
        scim, transactions, users,
    },
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    // Public auth routes (no JWT required)
    let public = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        // SSO initiation & callbacks (public — redirect-based flows)
        .route("/auth/oidc/{provider_id}", get(auth_sso::oidc_initiate))
        .route(
            "/auth/oidc/{provider_id}/callback",
            get(auth_sso::oidc_callback),
        )
        .route("/auth/saml/{provider_id}", get(auth_sso::saml_initiate))
        .route(
            "/auth/saml/{provider_id}/callback",
            post(auth_sso::saml_callback),
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
            get(contacts::get_contact).patch(contacts::update_contact),
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
        .route("/health", get(health_check))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "ok"
}
