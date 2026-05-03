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
        accounts, audit, auth, auth_sso, bank, budgets, contacts, custom_fields, email,
        exchange_rates, expenses, fixed_assets, fx, health, identity, inventory, invoices, notes,
        notifications, organizations, payments, products, projects, purchase_orders, recurring,
        reports, roles, scim, tags, tax_rates, time_entries, transactions, users, webhooks,
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
        .route("/invoices/:id/convert", post(invoices::convert_quote))
        .route("/invoices/:id/apply-credit", post(invoices::apply_credit))
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
        .route("/reports/cash-flow", get(reports::cash_flow))
        .route("/reports/budget-vs-actual", get(budgets::budget_vs_actual))
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
        // Products / services catalog
        .route(
            "/products",
            get(products::list_products).post(products::create_product),
        )
        .route(
            "/products/:id",
            get(products::get_product)
                .patch(products::update_product)
                .delete(products::delete_product),
        )
        // Budgets
        .route(
            "/budgets",
            get(budgets::list_budgets).post(budgets::create_budget),
        )
        .route(
            "/budgets/:id",
            get(budgets::get_budget)
                .patch(budgets::update_budget)
                .delete(budgets::delete_budget),
        )
        .route(
            "/budgets/:id/lines",
            axum::routing::put(budgets::upsert_budget_lines),
        )
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
        // Purchase orders
        .route(
            "/purchase-orders",
            get(purchase_orders::list_purchase_orders).post(purchase_orders::create_purchase_order),
        )
        .route(
            "/purchase-orders/:id",
            get(purchase_orders::get_purchase_order)
                .patch(purchase_orders::update_purchase_order)
                .delete(purchase_orders::delete_purchase_order),
        )
        .route(
            "/purchase-orders/:id/receive",
            post(purchase_orders::receive_purchase_order),
        )
        .route(
            "/purchase-orders/:id/lines",
            post(purchase_orders::add_po_line),
        )
        // Fixed assets
        .route(
            "/fixed-assets",
            get(fixed_assets::list_fixed_assets).post(fixed_assets::create_fixed_asset),
        )
        .route(
            "/fixed-assets/:id",
            get(fixed_assets::get_fixed_asset).patch(fixed_assets::update_fixed_asset),
        )
        .route(
            "/fixed-assets/:id/depreciate",
            post(fixed_assets::depreciate_asset),
        )
        .route(
            "/fixed-assets/:id/dispose",
            post(fixed_assets::dispose_asset),
        )
        .route("/fixed-assets/register", get(fixed_assets::asset_register))
        // Projects
        .route(
            "/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/projects/:id",
            get(projects::get_project)
                .patch(projects::update_project)
                .delete(projects::delete_project),
        )
        .route("/projects/:id/summary", get(projects::project_summary))
        // Time entries
        .route(
            "/time-entries",
            get(time_entries::list_time_entries).post(time_entries::create_time_entry),
        )
        .route(
            "/time-entries/:id",
            get(time_entries::get_time_entry)
                .patch(time_entries::update_time_entry)
                .delete(time_entries::delete_time_entry),
        )
        .route("/time-entries/bill", post(time_entries::bill_time_entries))
        .route("/time-entries/summary", get(time_entries::time_summary))
        // Webhooks
        .route(
            "/webhooks",
            get(webhooks::list_webhooks).post(webhooks::create_webhook),
        )
        .route(
            "/webhooks/:id",
            get(webhooks::get_webhook)
                .patch(webhooks::update_webhook)
                .delete(webhooks::delete_webhook),
        )
        // Exchange rates
        .route("/exchange-rates", get(exchange_rates::get_rate))
        // Bank accounts & reconciliation
        .route(
            "/bank-accounts",
            get(bank::list_bank_accounts).post(bank::create_bank_account),
        )
        .route(
            "/bank-accounts/:id",
            get(bank::get_bank_account).patch(bank::update_bank_account),
        )
        .route(
            "/bank-accounts/:id/transactions",
            get(bank::list_bank_transactions).post(bank::import_bank_transactions),
        )
        .route(
            "/bank-accounts/:id/reconciliation",
            get(bank::reconciliation_summary),
        )
        .route(
            "/bank-transactions/:id/match",
            post(bank::match_bank_transaction),
        )
        .route(
            "/bank-transactions/:id/exclude",
            post(bank::exclude_bank_transaction),
        )
        // Inventory
        .route(
            "/inventory",
            get(inventory::list_inventory).post(inventory::create_inventory_item),
        )
        .route("/inventory/low-stock", get(inventory::low_stock))
        .route(
            "/inventory/:product_id",
            get(inventory::get_inventory_item).patch(inventory::update_inventory_item),
        )
        .route(
            "/inventory/:product_id/adjust",
            post(inventory::adjust_inventory),
        )
        .route(
            "/inventory/:product_id/movements",
            get(inventory::inventory_movements),
        )
        // Custom fields
        .route(
            "/custom-fields",
            get(custom_fields::list_custom_fields).post(custom_fields::create_custom_field),
        )
        .route(
            "/custom-fields/:id",
            get(custom_fields::get_custom_field)
                .patch(custom_fields::update_custom_field)
                .delete(custom_fields::delete_custom_field),
        )
        .route(
            "/custom-fields/values/:entity_type/:entity_id",
            get(custom_fields::get_entity_custom_fields)
                .put(custom_fields::set_entity_custom_fields),
        )
        // FX gain/loss report
        .route("/reports/fx-summary", get(fx::fx_summary))
        // Tags
        .route("/tags", get(tags::list_tags).post(tags::create_tag))
        .route(
            "/tags/:id",
            get(tags::get_tag)
                .patch(tags::update_tag)
                .delete(tags::delete_tag),
        )
        .route("/:entity_type/:entity_id/tags", get(tags::list_entity_tags))
        .route(
            "/:entity_type/:entity_id/tags/:tag_id",
            post(tags::add_entity_tag).delete(tags::remove_entity_tag),
        )
        // Notes
        .route(
            "/:entity_type/:entity_id/notes",
            get(notes::list_notes).post(notes::create_note),
        )
        .route(
            "/:entity_type/:entity_id/notes/:id",
            delete(notes::delete_note),
        )
        // Notifications
        .route("/notifications", get(notifications::list_notifications))
        .route(
            "/notifications/:id/read",
            post(notifications::mark_notification_read),
        )
        .route(
            "/notifications/read-all",
            post(notifications::mark_all_notifications_read),
        )
        // Email settings & log
        .route(
            "/email-settings",
            get(email::get_email_settings).put(email::upsert_email_settings),
        )
        .route("/email-log", get(email::list_email_log))
        .route("/email/send", post(email::send_email))
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
