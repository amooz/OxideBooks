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
        accounts, api_keys, approval_rules, attachments, audit, auth, auth_sso, bank, bank_rules,
        batch_payments, bills, budgets, bulk, client_portal, closed_periods, consolidated,
        contact_groups, contacts, credit_notes, custom_fields, deferred_revenue, departments,
        doc_sequences, dunning, email, employees, exchange_rates, expense_categories,
        expense_policies, expense_reports, expenses, export, fixed_assets, fx, health, identity,
        import, inventory, invoice_templates, invoices, late_fees, leave, mileage, notes,
        notifications, opening_balances, organizations, payment_links, payment_plans,
        payment_terms, payments, payroll, payslips, prepayments, price_lists, product_categories,
        products, projects, purchase_orders, purchase_requisitions, recurring, reports, retainers,
        roles, sales_orders, scim, sessions, stripe_webhook, subscriptions, tags, tax_groups,
        tax_periods, tax_rates, time_entries, totp, tracking_categories, transactions, users,
        vendor_credits, webhooks,
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
        )
        // Public payment link view (no JWT)
        .route("/pay/:token", get(payment_links::view_payment_link))
        // Client portal — read-only invoice view for contacts (no JWT)
        .route("/portal/:token", get(client_portal::portal_view))
        .route(
            "/portal/:token/invoices/:invoice_id",
            get(client_portal::portal_invoice),
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
        .route(
            "/transactions/:id/reverse",
            post(transactions::reverse_transaction),
        )
        .route(
            "/transactions/:id/submit",
            post(transactions::submit_transaction),
        )
        .route(
            "/transactions/:id/approve",
            post(transactions::approve_transaction),
        )
        // Contact groups
        .route(
            "/contact-groups",
            get(contact_groups::list_contact_groups).post(contact_groups::create_contact_group),
        )
        .route(
            "/contact-groups/:id",
            get(contact_groups::get_contact_group)
                .patch(contact_groups::update_contact_group)
                .delete(contact_groups::delete_contact_group),
        )
        .route(
            "/contact-groups/:id/members",
            get(contact_groups::list_group_members),
        )
        .route(
            "/contact-groups/:id/members/:contact_id",
            post(contact_groups::add_group_member).delete(contact_groups::remove_group_member),
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
        // Payment void and refunds
        .route("/payments/:id/void", post(payments::void_payment))
        .route(
            "/payments/:id/refunds",
            post(payments::create_refund).get(payments::list_refunds),
        )
        .route("/invoices/:id/convert", post(invoices::convert_quote))
        // Quote workflow
        .route("/quotes/:id/accept", post(invoices::accept_quote))
        .route("/quotes/:id/decline", post(invoices::decline_quote))
        .route("/quotes/:id/expire", post(invoices::expire_quote))
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
        .route("/reports/1099-summary", get(reports::summary_1099))
        .route("/reports/account-ledger", get(reports::account_ledger))
        .route("/reports/sales-by-product", get(reports::sales_by_product))
        .route(
            "/reports/project-profitability",
            get(reports::project_profitability),
        )
        // Dashboard
        .route("/dashboard", get(reports::dashboard))
        // Global search
        .route("/search", get(reports::global_search))
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
        // Tax filing periods
        .route(
            "/tax-periods",
            get(tax_periods::list_tax_periods).post(tax_periods::create_tax_period),
        )
        .route(
            "/tax-periods/:id",
            get(tax_periods::get_tax_period).delete(tax_periods::delete_tax_period),
        )
        .route("/tax-periods/:id/file", post(tax_periods::file_tax_period))
        .route("/tax-periods/:id/lock", post(tax_periods::lock_tax_period))
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
        // Expense categories
        .route(
            "/expense-categories",
            get(expense_categories::list_expense_categories)
                .post(expense_categories::create_expense_category),
        )
        .route(
            "/expense-categories/:id",
            get(expense_categories::get_expense_category)
                .patch(expense_categories::update_expense_category)
                .delete(expense_categories::delete_expense_category),
        )
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
        // Product categories
        .route(
            "/product-categories",
            get(product_categories::list_product_categories)
                .post(product_categories::create_product_category),
        )
        .route(
            "/product-categories/:id",
            get(product_categories::get_product_category)
                .patch(product_categories::update_product_category)
                .delete(product_categories::delete_product_category),
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
            "/recurring-schedules/run-due",
            post(recurring::run_due_schedules),
        )
        .route(
            "/recurring-schedules/:id",
            get(recurring::get_schedule)
                .patch(recurring::update_schedule)
                .delete(recurring::delete_schedule),
        )
        .route(
            "/recurring-schedules/:id/run",
            post(recurring::run_schedule),
        )
        // Approval workflow rules
        .route(
            "/approval-rules",
            get(approval_rules::list_approval_rules).post(approval_rules::create_approval_rule),
        )
        .route(
            "/approval-rules/:id",
            get(approval_rules::get_approval_rule)
                .patch(approval_rules::update_approval_rule)
                .delete(approval_rules::delete_approval_rule),
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
        .route(
            "/purchase-orders/:id/create-bill",
            post(purchase_orders::po_create_bill),
        )
        .route(
            "/purchase-orders/:id/approve",
            post(purchase_orders::approve_purchase_order),
        )
        // Purchase requisitions
        .route(
            "/purchase-requisitions",
            get(purchase_requisitions::list_purchase_requisitions)
                .post(purchase_requisitions::create_purchase_requisition),
        )
        .route(
            "/purchase-requisitions/:id",
            get(purchase_requisitions::get_purchase_requisition)
                .patch(purchase_requisitions::update_purchase_requisition),
        )
        .route(
            "/purchase-requisitions/:id/submit",
            post(purchase_requisitions::submit_purchase_requisition),
        )
        .route(
            "/purchase-requisitions/:id/approve",
            post(purchase_requisitions::approve_purchase_requisition),
        )
        .route(
            "/purchase-requisitions/:id/reject",
            post(purchase_requisitions::reject_purchase_requisition),
        )
        .route(
            "/purchase-requisitions/:id/convert",
            post(purchase_requisitions::convert_requisition_to_po),
        )
        // Document number sequences
        .route(
            "/doc-sequences",
            get(doc_sequences::list_sequences).put(doc_sequences::upsert_sequence),
        )
        .route(
            "/doc-sequences/:doc_type/reset",
            post(doc_sequences::reset_sequence),
        )
        // Expense reports
        .route(
            "/expense-reports",
            get(expense_reports::list_expense_reports).post(expense_reports::create_expense_report),
        )
        .route(
            "/expense-reports/:id",
            get(expense_reports::get_expense_report).patch(expense_reports::update_expense_report),
        )
        .route(
            "/expense-reports/:id/expenses",
            post(expense_reports::add_expense_to_report),
        )
        .route(
            "/expense-reports/:id/submit",
            post(expense_reports::submit_expense_report),
        )
        .route(
            "/expense-reports/:id/approve",
            post(expense_reports::approve_expense_report),
        )
        .route(
            "/expense-reports/:id/reject",
            post(expense_reports::reject_expense_report),
        )
        .route(
            "/expense-reports/:id/reimburse",
            post(expense_reports::reimburse_expense_report),
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
        // Bank rules
        .route(
            "/bank-rules",
            get(bank_rules::list_bank_rules).post(bank_rules::create_bank_rule),
        )
        .route("/bank-rules/:id", delete(bank_rules::delete_bank_rule))
        .route(
            "/bank-accounts/:id/apply-rules",
            post(bank_rules::apply_bank_rules),
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
            "/reports/inventory-valuation",
            get(inventory::inventory_valuation),
        )
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
        // Attachments
        .route(
            "/:entity_type/:entity_id/attachments",
            get(attachments::list_attachments).post(attachments::create_attachment),
        )
        .route(
            "/:entity_type/:entity_id/attachments/:id",
            delete(attachments::delete_attachment),
        )
        // Payment links
        .route(
            "/payment-links",
            get(payment_links::list_payment_links).post(payment_links::create_payment_link),
        )
        .route(
            "/payment-links/:id",
            get(payment_links::get_payment_link).delete(payment_links::cancel_payment_link),
        )
        // Batch payments
        .route(
            "/batch-payments",
            get(batch_payments::list_batch_payments).post(batch_payments::create_batch_payment),
        )
        .route(
            "/batch-payments/:id",
            get(batch_payments::get_batch_payment),
        )
        // Departments / cost centres
        .route(
            "/departments",
            get(departments::list_departments).post(departments::create_department),
        )
        .route(
            "/departments/:id",
            axum::routing::patch(departments::update_department)
                .delete(departments::delete_department),
        )
        .route("/departments/:id/pl", get(departments::department_pl))
        // Invoice branding template
        .route(
            "/invoice-template",
            get(invoice_templates::get_invoice_template)
                .put(invoice_templates::upsert_invoice_template),
        )
        // Payroll
        .route(
            "/payroll-runs",
            get(payroll::list_payroll_runs).post(payroll::create_payroll_run),
        )
        .route("/payroll-runs/:id", get(payroll::get_payroll_run))
        .route(
            "/payroll-runs/:id/entries",
            post(payroll::add_payroll_entry),
        )
        .route(
            "/payroll-runs/:id/approve",
            post(payroll::approve_payroll_run),
        )
        .route("/payroll-runs/:id/pay", post(payroll::pay_payroll_run))
        // CSV exports
        .route("/export/invoices", get(export::export_invoices))
        .route("/export/expenses", get(export::export_expenses))
        .route("/export/transactions", get(export::export_transactions))
        .route("/export/profit-loss", get(export::export_profit_loss))
        .route("/export/trial-balance", get(export::export_trial_balance))
        .route("/export/accounts", get(export::export_accounts))
        // Consolidated (multi-entity) reports
        .route(
            "/reports/consolidated",
            get(consolidated::consolidated_profit_loss),
        )
        // Dunning (overdue reminders)
        .route(
            "/dunning-rules",
            get(dunning::list_dunning_rules).post(dunning::upsert_dunning_rule),
        )
        .route("/dunning-rules/:id", delete(dunning::delete_dunning_rule))
        .route("/invoices/overdue", get(dunning::list_overdue_invoices))
        .route(
            "/invoices/:id/reminders",
            get(dunning::list_invoice_reminders).post(dunning::send_reminder),
        )
        // Mileage tracking
        .route(
            "/mileage",
            get(mileage::list_mileage_trips).post(mileage::create_mileage_trip),
        )
        .route("/mileage/summary", get(mileage::mileage_summary))
        .route("/mileage/:id", delete(mileage::delete_mileage_trip))
        // Contacts statement
        .route("/contacts/:id/statement", get(contacts::contact_statement))
        // Expense policies
        .route(
            "/expense-policies",
            get(expense_policies::list_expense_policies),
        )
        .route(
            "/expense-policies/:category",
            axum::routing::put(expense_policies::upsert_expense_policy)
                .delete(expense_policies::delete_expense_policy),
        )
        // CSV import
        .route("/import/contacts", post(import::import_contacts_csv))
        .route("/import/accounts", post(import::import_accounts_csv))
        // Late fees
        .route(
            "/late-fee-rule",
            get(late_fees::get_late_fee_rule).put(late_fees::upsert_late_fee_rule),
        )
        .route("/invoices/:id/late-fee", post(late_fees::apply_late_fee))
        .route("/invoices/:id/late-fees", get(late_fees::list_late_fees))
        // Retainers
        .route(
            "/retainers",
            get(retainers::list_retainers).post(retainers::create_retainer),
        )
        .route("/retainers/:id/deposit", post(retainers::deposit_retainer))
        .route("/retainers/:id/apply", post(retainers::apply_retainer))
        .route(
            "/retainers/:id/transactions",
            get(retainers::list_retainer_transactions),
        )
        // Client portal token creation
        .route("/portal-tokens", post(client_portal::create_portal_token))
        // Bulk operations
        .route("/invoices/bulk-void", post(bulk::bulk_void_invoices))
        .route("/invoices/bulk-send", post(bulk::bulk_send_invoices))
        .route("/expenses/bulk-approve", post(bulk::bulk_approve_expenses))
        .route("/expenses/bulk-reject", post(bulk::bulk_reject_expenses))
        // Session management
        .route(
            "/sessions",
            get(sessions::list_sessions).delete(sessions::revoke_all_sessions),
        )
        .route("/sessions/:id", delete(sessions::revoke_session))
        // Closed accounting periods
        .route(
            "/closed-periods",
            get(closed_periods::list_closed_periods).post(closed_periods::close_period),
        )
        .route("/closed-periods/:id", delete(closed_periods::reopen_period))
        // API key management
        .route(
            "/api-keys",
            get(api_keys::list_api_keys).post(api_keys::create_api_key),
        )
        .route("/api-keys/:id/revoke", post(api_keys::revoke_api_key))
        // Payment terms
        .route(
            "/payment-terms",
            get(payment_terms::list_payment_terms).post(payment_terms::create_payment_terms),
        )
        .route(
            "/payment-terms/:id",
            get(payment_terms::get_payment_terms)
                .patch(payment_terms::update_payment_terms)
                .delete(payment_terms::delete_payment_terms),
        )
        // Opening balances
        .route(
            "/opening-balances",
            get(opening_balances::get_opening_balances)
                .post(opening_balances::set_opening_balances),
        )
        // Prepayments (customer advance payments)
        .route(
            "/prepayments",
            get(prepayments::list_prepayments).post(prepayments::create_prepayment),
        )
        .route("/prepayments/:id", get(prepayments::get_prepayment))
        .route(
            "/prepayments/:id/apply",
            post(prepayments::apply_prepayment),
        )
        .route("/prepayments/:id/void", post(prepayments::void_prepayment))
        // Price lists
        .route(
            "/price-lists",
            get(price_lists::list_price_lists).post(price_lists::create_price_list),
        )
        .route("/price-lists/:id", delete(price_lists::delete_price_list))
        .route(
            "/price-lists/:id/items",
            get(price_lists::list_price_list_items).put(price_lists::upsert_price_list_item),
        )
        // Spend analysis
        .route("/reports/spend-analysis", get(price_lists::spend_analysis))
        // Employees
        .route(
            "/employees",
            get(employees::list_employees).post(employees::create_employee),
        )
        .route(
            "/employees/:id",
            get(employees::get_employee)
                .patch(employees::update_employee)
                .delete(employees::delete_employee),
        )
        .route(
            "/employees/:id/leave-balance",
            get(leave::employee_leave_balance),
        )
        // Vendor bills (AP)
        .route("/bills", get(bills::list_bills).post(bills::create_bill))
        .route("/bills/:id", get(bills::get_bill).patch(bills::update_bill))
        .route("/bills/:id/approve", post(bills::approve_bill))
        .route("/bills/:id/void", post(bills::void_bill))
        .route(
            "/bills/:id/payments",
            post(bills::create_bill_payment).get(bills::list_bill_payments),
        )
        // Vendor credits (AP credit memos)
        .route(
            "/vendor-credits",
            get(vendor_credits::list_vendor_credits).post(vendor_credits::create_vendor_credit),
        )
        .route(
            "/vendor-credits/:id",
            get(vendor_credits::get_vendor_credit),
        )
        .route(
            "/vendor-credits/:id/void",
            post(vendor_credits::void_vendor_credit),
        )
        .route(
            "/vendor-credits/:id/apply",
            post(vendor_credits::apply_vendor_credit),
        )
        .route(
            "/vendor-credits/:id/applications",
            get(vendor_credits::list_credit_applications),
        )
        // Credit notes
        .route(
            "/credit-notes",
            get(credit_notes::list_credit_notes).post(credit_notes::create_credit_note),
        )
        .route("/credit-notes/:id", get(credit_notes::get_credit_note))
        .route(
            "/credit-notes/:id/apply",
            post(credit_notes::apply_credit_note),
        )
        .route(
            "/credit-notes/:id/void",
            post(credit_notes::void_credit_note),
        )
        .route(
            "/credit-notes/:id/applications",
            get(credit_notes::list_credit_note_applications),
        )
        // Payslips
        .route(
            "/payroll-runs/:id/payslips",
            post(payslips::create_payslip).get(payslips::list_payslips),
        )
        .route("/payslips/:id", get(payslips::get_payslip))
        // Leave management
        .route(
            "/leave-types",
            get(leave::list_leave_types).post(leave::create_leave_type),
        )
        .route(
            "/leave-types/:id",
            axum::routing::patch(leave::update_leave_type).delete(leave::delete_leave_type),
        )
        .route(
            "/leave-requests",
            get(leave::list_leave_requests).post(leave::create_leave_request),
        )
        .route(
            "/leave-requests/:id/approve",
            post(leave::approve_leave_request),
        )
        .route(
            "/leave-requests/:id/reject",
            post(leave::reject_leave_request),
        )
        .route(
            "/leave-requests/:id/cancel",
            post(leave::cancel_leave_request),
        )
        // TOTP 2FA
        .route("/auth/totp/setup", post(totp::setup_totp))
        .route("/auth/totp/verify", post(totp::verify_totp))
        .route("/auth/totp", delete(totp::disable_totp))
        // Sales orders
        .route(
            "/sales-orders",
            get(sales_orders::list_sales_orders).post(sales_orders::create_sales_order),
        )
        .route(
            "/sales-orders/:id",
            get(sales_orders::get_sales_order).patch(sales_orders::update_sales_order),
        )
        .route(
            "/sales-orders/:id/confirm",
            post(sales_orders::confirm_sales_order),
        )
        .route(
            "/sales-orders/:id/cancel",
            post(sales_orders::cancel_sales_order),
        )
        .route(
            "/sales-orders/:id/convert-to-invoice",
            post(sales_orders::convert_so_to_invoice),
        )
        // Tax groups
        .route(
            "/tax-groups",
            get(tax_groups::list_tax_groups).post(tax_groups::create_tax_group),
        )
        .route(
            "/tax-groups/:id",
            get(tax_groups::get_tax_group)
                .patch(tax_groups::update_tax_group)
                .delete(tax_groups::delete_tax_group),
        )
        // Deferred revenue schedules
        .route(
            "/deferred-revenue",
            get(deferred_revenue::list_schedules).post(deferred_revenue::create_schedule),
        )
        .route("/deferred-revenue/:id", get(deferred_revenue::get_schedule))
        .route(
            "/deferred-revenue/:id/entries/:entry_id/recognize",
            post(deferred_revenue::recognize_entry),
        )
        .route(
            "/deferred-revenue/:id/cancel",
            post(deferred_revenue::cancel_schedule),
        )
        // Payment plans
        .route(
            "/payment-plans",
            get(payment_plans::list_payment_plans).post(payment_plans::create_payment_plan),
        )
        .route("/payment-plans/:id", get(payment_plans::get_payment_plan))
        .route(
            "/payment-plans/:id/installments/:inst_id/pay",
            post(payment_plans::pay_installment),
        )
        .route(
            "/payment-plans/:id/cancel",
            post(payment_plans::cancel_payment_plan),
        )
        // Subscription plans
        .route(
            "/subscription-plans",
            get(subscriptions::list_plans).post(subscriptions::create_plan),
        )
        .route(
            "/subscription-plans/:id",
            get(subscriptions::get_plan).patch(subscriptions::update_plan),
        )
        // Subscriptions
        .route(
            "/subscriptions",
            get(subscriptions::list_subscriptions).post(subscriptions::create_subscription),
        )
        .route(
            "/subscriptions/:id",
            get(subscriptions::get_subscription).patch(subscriptions::update_subscription),
        )
        .route(
            "/subscriptions/:id/cancel",
            post(subscriptions::cancel_subscription),
        )
        .route(
            "/subscriptions/:id/renew",
            post(subscriptions::renew_subscription),
        )
        // Tracking categories (QB-style classes/locations)
        .route(
            "/tracking-categories",
            get(tracking_categories::list_tracking_categories)
                .post(tracking_categories::create_tracking_category),
        )
        .route(
            "/tracking-categories/:id",
            get(tracking_categories::get_tracking_category)
                .patch(tracking_categories::update_tracking_category)
                .delete(tracking_categories::delete_tracking_category),
        )
        .route(
            "/tracking-categories/:id/options",
            post(tracking_categories::add_tracking_option),
        )
        .route(
            "/tracking-categories/:id/options/:option_id",
            axum::routing::patch(tracking_categories::update_tracking_option)
                .delete(tracking_categories::delete_tracking_option),
        )
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
        // Stripe webhook (public — HMAC-verified internally)
        .route("/webhooks/stripe", post(stripe_webhook::stripe_webhook))
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
