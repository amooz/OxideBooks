use std::{sync::Arc, time::Duration};

use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{delete, get, post, put},
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
        accounts, api_keys, approval_rules, assembly_orders, attachments, audit, auth, auth_sso,
        bank, bank_deposits, bank_reconciliation, bank_rules, batch_payments, bills, budgets, bulk,
        check_runs, client_portal, closed_periods, commissions, consolidated,
        consolidation_eliminations, contact_groups, contacts, contractor_tax_info, cost_codes,
        credit_notes, custom_fields, deferred_charges, deferred_revenue, departments,
        direct_deposit, doc_sequences, dunning, email, employee_bank_accounts, employee_loans,
        employees, exchange_rates, expense_categories, expense_claims, expense_policies,
        expense_reports, expenses, export, fixed_assets, fx, fx_revaluations, grn, health,
        identity, import, intercompany, inventory, inventory_lots, inventory_reorder_requests,
        inventory_serial_numbers, inventory_stocktakes, invoice_templates, invoices, landed_costs,
        late_fees, leave, mileage, notes, notifications, opening_balances, organizations,
        payment_links, payment_plans, payment_terms, payments, payroll, payroll_tax, payslips,
        prepaid_expenses, prepayments, price_lists, product_categories, product_variants, products,
        project_phases, projects, purchase_orders, purchase_requisitions, recurring,
        recurring_bills, recurring_invoices, recurring_journal_entries, reports, retainers, roles,
        sales_orders, sales_tax_nexus, scim, service_territories, sessions, stripe_webhook,
        subscriptions, tags, tax_groups, tax_periods, tax_rates, tax_rules, time_entries, totp,
        tracking_categories, transactions, users, vendor_credits, vendor_portal, warehouses,
        webhooks, work_orders,
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
        )
        // Vendor portal (public — token-based)
        .route(
            "/vendor-portal/:token",
            get(vendor_portal::vendor_portal_view),
        )
        .route(
            "/vendor-portal/:token/bills/:bill_id",
            get(vendor_portal::vendor_portal_bill),
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
        .route(
            "/transactions/auto-reversals",
            post(transactions::process_auto_reversals),
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
        .route("/invoices/cash-sale", post(invoices::create_cash_sale))
        .route(
            "/invoices/from-expenses",
            post(invoices::create_from_expenses),
        )
        .route(
            "/invoices/:id",
            get(invoices::get_invoice).patch(invoices::update_invoice),
        )
        .route(
            "/invoices/:id/payments",
            post(payments::create_payment).get(payments::list_payments),
        )
        // Payment void, refunds, and FX journal
        .route("/payments/:id/void", post(payments::void_payment))
        .route(
            "/payments/:id/refunds",
            post(payments::create_refund).get(payments::list_refunds),
        )
        .route("/payments/:id/fx-journal", post(payments::post_fx_journal))
        .route("/invoices/:id/convert", post(invoices::convert_quote))
        // Quote workflow
        .route("/quotes/:id/accept", post(invoices::accept_quote))
        .route("/quotes/:id/decline", post(invoices::decline_quote))
        .route("/quotes/:id/expire", post(invoices::expire_quote))
        .route(
            "/quotes/:id/progress-invoice",
            post(invoices::progress_invoice),
        )
        .route("/invoices/:id/apply-credit", post(invoices::apply_credit))
        // Sales commissions
        .route("/commissions", get(commissions::list_commissions))
        .route("/commissions/:id", get(commissions::get_commission))
        .route(
            "/commissions/:id/approve",
            post(commissions::approve_commission),
        )
        .route("/commissions/:id/pay", post(commissions::pay_commission))
        .route("/commissions/:id/void", post(commissions::void_commission))
        .route(
            "/invoices/:id/commissions",
            get(commissions::list_invoice_commissions).post(commissions::create_commission),
        )
        // Intercompany (multi-entity)
        .route(
            "/intercompany/links",
            get(intercompany::list_links).post(intercompany::create_link),
        )
        .route("/intercompany/links/:id", delete(intercompany::delete_link))
        .route(
            "/intercompany/transactions",
            get(intercompany::list_transactions).post(intercompany::create_transaction),
        )
        // Consolidation eliminations
        .route(
            "/consolidation-eliminations",
            get(consolidation_eliminations::list_eliminations)
                .post(consolidation_eliminations::create_elimination),
        )
        .route(
            "/consolidation-eliminations/:id",
            get(consolidation_eliminations::get_elimination),
        )
        .route(
            "/consolidation-eliminations/:id/void",
            post(consolidation_eliminations::void_elimination),
        )
        // Contractor 1099 tax info
        .route(
            "/contractor-tax-info",
            get(contractor_tax_info::list_contractor_tax_info)
                .post(contractor_tax_info::create_contractor_tax_info),
        )
        .route(
            "/contractor-tax-info/:id",
            get(contractor_tax_info::get_contractor_tax_info)
                .patch(contractor_tax_info::update_contractor_tax_info),
        )
        .route(
            "/contacts/:id/contractor-tax-info",
            get(contractor_tax_info::get_contact_contractor_tax_info),
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
        .route(
            "/reports/balance-sheet-comparison",
            get(reports::balance_sheet_comparison),
        )
        .route("/reports/aging", get(reports::aging))
        .route("/reports/tax-summary", get(reports::tax_summary))
        .route("/reports/cash-flow", get(reports::cash_flow))
        .route(
            "/reports/cash-flow-forecast",
            get(reports::cash_flow_forecast),
        )
        .route("/reports/budget-vs-actual", get(budgets::budget_vs_actual))
        .route("/reports/1099-summary", get(reports::summary_1099))
        .route(
            "/reports/1099-payments",
            get(contractor_tax_info::report_1099_payments),
        )
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
        .route("/expenses/billable", get(expenses::list_billable_expenses))
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
        .route(
            "/products/:id/bundle-components",
            put(products::set_bundle_components),
        )
        // Product variants
        .route(
            "/products/:id/variants",
            get(product_variants::list_variants).post(product_variants::create_variant),
        )
        .route(
            "/products/:id/variants/:vid",
            get(product_variants::get_variant)
                .patch(product_variants::update_variant)
                .delete(product_variants::delete_variant),
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
        // Recurring journal entries (memorized transactions)
        .route(
            "/recurring-journal-entries",
            get(recurring_journal_entries::list_recurring_journal_entries)
                .post(recurring_journal_entries::create_recurring_journal_entry),
        )
        .route(
            "/recurring-journal-entries/:id",
            get(recurring_journal_entries::get_recurring_journal_entry)
                .patch(recurring_journal_entries::update_recurring_journal_entry)
                .delete(recurring_journal_entries::delete_recurring_journal_entry),
        )
        .route(
            "/recurring-journal-entries/:id/post",
            post(recurring_journal_entries::post_recurring_journal_entry),
        )
        // Recurring vendor bills
        .route(
            "/recurring-bills",
            get(recurring_bills::list_recurring_bills).post(recurring_bills::create_recurring_bill),
        )
        .route(
            "/recurring-bills/:id",
            get(recurring_bills::get_recurring_bill)
                .patch(recurring_bills::update_recurring_bill)
                .delete(recurring_bills::delete_recurring_bill),
        )
        .route(
            "/recurring-bills/:id/generate",
            post(recurring_bills::generate_recurring_bill),
        )
        .route(
            "/recurring-invoices",
            get(recurring_invoices::list_recurring_invoices)
                .post(recurring_invoices::create_recurring_invoice),
        )
        .route(
            "/recurring-invoices/:id",
            get(recurring_invoices::get_recurring_invoice)
                .patch(recurring_invoices::update_recurring_invoice)
                .delete(recurring_invoices::delete_recurring_invoice),
        )
        .route(
            "/recurring-invoices/:id/generate",
            post(recurring_invoices::generate_recurring_invoice),
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
        .route(
            "/purchase-orders/:id/receipts",
            get(grn::list_receipts).post(grn::create_receipt),
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
        // Expense claims
        .route(
            "/expense-claims",
            get(expense_claims::list_expense_claims).post(expense_claims::create_expense_claim),
        )
        .route(
            "/expense-claims/:id",
            get(expense_claims::get_expense_claim).patch(expense_claims::update_expense_claim),
        )
        .route(
            "/expense-claims/:id/submit",
            post(expense_claims::submit_expense_claim),
        )
        .route(
            "/expense-claims/:id/approve",
            post(expense_claims::approve_expense_claim),
        )
        .route(
            "/expense-claims/:id/reject",
            post(expense_claims::reject_expense_claim),
        )
        .route(
            "/expense-claims/:id/reimburse",
            post(expense_claims::reimburse_expense_claim),
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
        // Project phases
        .route(
            "/projects/:id/phases",
            get(project_phases::list_phases).post(project_phases::create_phase),
        )
        .route(
            "/project-phases/:id",
            get(project_phases::get_phase)
                .patch(project_phases::update_phase)
                .delete(project_phases::delete_phase),
        )
        // Assembly orders
        .route(
            "/assembly-orders",
            get(assembly_orders::list_assembly_orders).post(assembly_orders::create_assembly_order),
        )
        .route(
            "/assembly-orders/:id",
            get(assembly_orders::get_assembly_order),
        )
        .route(
            "/assembly-orders/:id/lines",
            get(assembly_orders::list_assembly_order_lines),
        )
        .route(
            "/assembly-orders/:id/build",
            post(assembly_orders::build_assembly_order),
        )
        .route(
            "/assembly-orders/:id/cancel",
            post(assembly_orders::cancel_assembly_order),
        )
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
        // Bank deposits
        .route(
            "/bank-deposits",
            get(bank_deposits::list_bank_deposits).post(bank_deposits::create_bank_deposit),
        )
        .route(
            "/bank-deposits/:id",
            get(bank_deposits::get_bank_deposit).delete(bank_deposits::delete_bank_deposit),
        )
        .route(
            "/bank-deposits/:id/clear",
            post(bank_deposits::clear_bank_deposit),
        )
        // Bank reconciliation statements
        .route(
            "/bank-reconciliation-statements",
            get(bank_reconciliation::list_reconciliation_statements)
                .post(bank_reconciliation::create_reconciliation_statement),
        )
        .route(
            "/bank-reconciliation-statements/:id",
            get(bank_reconciliation::get_reconciliation_statement)
                .delete(bank_reconciliation::delete_reconciliation_statement),
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
        .route(
            "/inventory/:item_id/lots",
            get(inventory_lots::list_lots).post(inventory_lots::create_lot),
        )
        // Inventory lot routes keyed on lot id (no item context needed)
        .route(
            "/inventory/lots/expiring",
            get(inventory_lots::list_expiring),
        )
        .route(
            "/inventory/lots/:id",
            get(inventory_lots::get_lot).patch(inventory_lots::update_lot),
        )
        // Goods receipt notes
        .route("/goods-receipts/:id", get(grn::get_receipt))
        .route("/goods-receipts/:id/post", post(grn::post_receipt))
        .route(
            "/goods-receipts/:id/landed-costs",
            get(landed_costs::list_landed_costs).post(landed_costs::create_landed_cost),
        )
        // Warehouses
        .route(
            "/warehouses",
            get(warehouses::list_warehouses).post(warehouses::create_warehouse),
        )
        .route("/warehouses/transfer", post(warehouses::transfer_stock))
        .route(
            "/warehouses/:id",
            get(warehouses::get_warehouse)
                .patch(warehouses::update_warehouse)
                .delete(warehouses::delete_warehouse),
        )
        .route(
            "/warehouses/:id/stock",
            get(warehouses::get_warehouse_stock),
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
        // FX revaluations (period-end unrealized gain/loss)
        .route(
            "/fx/revaluations",
            get(fx_revaluations::list_revaluations).post(fx_revaluations::create_revaluation),
        )
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
        .route(
            "/batch-payments/:id/remittance",
            get(batch_payments::remittance_advice),
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
        // Direct deposit / ACH batches
        .route(
            "/direct-deposit-batches",
            get(direct_deposit::list_batches).post(direct_deposit::create_batch),
        )
        .route(
            "/direct-deposit-batches/:id",
            get(direct_deposit::get_batch).delete(direct_deposit::delete_batch),
        )
        .route(
            "/direct-deposit-batches/:id/send",
            axum::routing::post(direct_deposit::mark_sent),
        )
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
        .route(
            "/payroll-runs/:id/post-journal",
            post(payroll::post_payroll_journal),
        )
        .route(
            "/payroll-runs/:id/tax-liabilities",
            get(payroll_tax::list_run_liabilities),
        )
        // Payroll tax liabilities
        .route(
            "/payroll-tax-liabilities",
            get(payroll_tax::list_liabilities).post(payroll_tax::create_liability),
        )
        .route(
            "/payroll-tax-liabilities/:id",
            get(payroll_tax::get_liability),
        )
        .route(
            "/payroll-tax-liabilities/:id/pay",
            post(payroll_tax::pay_liability),
        )
        .route(
            "/payroll-tax-liabilities/:id/void",
            post(payroll_tax::void_liability),
        )
        // CSV exports
        .route("/export/invoices", get(export::export_invoices))
        .route("/export/expenses", get(export::export_expenses))
        .route("/export/transactions", get(export::export_transactions))
        .route("/export/profit-loss", get(export::export_profit_loss))
        .route("/export/trial-balance", get(export::export_trial_balance))
        .route("/export/accounts", get(export::export_accounts))
        .route("/export/w2-data", get(export::export_w2_data))
        .route("/export/941-data", get(export::export_941_data))
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
        .route("/contacts/:id/merge", post(contacts::merge_contact))
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
        // Vendor portal token management
        .route(
            "/vendor-portal/tokens",
            post(vendor_portal::create_vendor_portal_token),
        )
        .route(
            "/vendor-portal/tokens/:contact_id",
            axum::routing::delete(vendor_portal::revoke_vendor_portal_token),
        )
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
        // Prepaid expense amortization schedules
        .route(
            "/prepaid-expenses",
            get(prepaid_expenses::list_schedules).post(prepaid_expenses::create_schedule),
        )
        .route(
            "/prepaid-expenses/:id",
            get(prepaid_expenses::get_schedule).patch(prepaid_expenses::update_schedule),
        )
        .route(
            "/prepaid-expenses/entries/:id/recognize",
            post(prepaid_expenses::recognize_entry),
        )
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
        .route("/reports/job-costing", get(reports::job_costing))
        .route("/reports/vendor-spend", get(reports::vendor_spend))
        .route("/reports/pl-comparison", get(reports::pl_comparison))
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
        .route(
            "/employees/:id/bank-accounts",
            get(employee_bank_accounts::list_employee_bank_accounts)
                .post(employee_bank_accounts::create_employee_bank_account),
        )
        .route(
            "/employee-bank-accounts/:id",
            axum::routing::patch(employee_bank_accounts::update_employee_bank_account)
                .delete(employee_bank_accounts::delete_employee_bank_account),
        )
        .route(
            "/employees/:id/loans",
            get(employee_loans::list_employee_loans),
        )
        // Employee loans
        .route(
            "/employee-loans",
            get(employee_loans::list_loans).post(employee_loans::create_loan),
        )
        .route(
            "/employee-loans/:id",
            get(employee_loans::get_loan).patch(employee_loans::update_loan),
        )
        .route(
            "/employee-loans/:id/repayments",
            get(employee_loans::list_repayments).post(employee_loans::create_repayment),
        )
        .route(
            "/employee-loans/:id/write-off",
            post(employee_loans::write_off_loan),
        )
        // Vendor bills (AP)
        .route("/bills", get(bills::list_bills).post(bills::create_bill))
        .route("/bills/spend-money", post(bills::create_spend_money))
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
        // Sales tax nexus
        .route(
            "/sales-tax-nexus",
            get(sales_tax_nexus::list_nexus).post(sales_tax_nexus::create_nexus),
        )
        .route(
            "/sales-tax-nexus/:id",
            get(sales_tax_nexus::get_nexus)
                .patch(sales_tax_nexus::update_nexus)
                .delete(sales_tax_nexus::delete_nexus),
        )
        // Check runs
        .route(
            "/check-runs",
            get(check_runs::list_check_runs).post(check_runs::create_check_run),
        )
        .route("/check-runs/:id", get(check_runs::get_check_run))
        .route(
            "/check-runs/:id/items",
            get(check_runs::list_check_run_items),
        )
        .route("/check-runs/:id/print", post(check_runs::print_check_run))
        .route("/check-runs/:id/void", post(check_runs::void_check_run))
        .route(
            "/check-runs/:id/items/:item_id/void",
            post(check_runs::void_check_run_item),
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
        // Deferred charges (bill-later / progress charge capture)
        .route(
            "/deferred-charges",
            get(deferred_charges::list_deferred_charges)
                .post(deferred_charges::create_deferred_charge),
        )
        .route(
            "/deferred-charges/:id",
            get(deferred_charges::get_deferred_charge)
                .patch(deferred_charges::update_deferred_charge),
        )
        .route(
            "/deferred-charges/:id/void",
            post(deferred_charges::void_deferred_charge),
        )
        .route(
            "/deferred-charges/:id/invoice",
            post(deferred_charges::invoice_deferred_charges),
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
        // Work orders (service tickets)
        .route(
            "/work-orders",
            get(work_orders::list_work_orders).post(work_orders::create_work_order),
        )
        .route(
            "/work-orders/:id",
            get(work_orders::get_work_order)
                .patch(work_orders::update_work_order)
                .delete(work_orders::delete_work_order),
        )
        .route(
            "/work-orders/:id/start",
            post(work_orders::start_work_order),
        )
        .route("/work-orders/:id/hold", post(work_orders::hold_work_order))
        .route(
            "/work-orders/:id/complete",
            post(work_orders::complete_work_order),
        )
        .route(
            "/work-orders/:id/cancel",
            post(work_orders::cancel_work_order),
        )
        // Cost codes (job costing)
        .route(
            "/cost-codes",
            get(cost_codes::list_cost_codes).post(cost_codes::create_cost_code),
        )
        .route(
            "/cost-codes/:id",
            get(cost_codes::get_cost_code)
                .patch(cost_codes::update_cost_code)
                .delete(cost_codes::delete_cost_code),
        )
        // Inventory reorder requests
        .route(
            "/inventory-reorder-requests",
            get(inventory_reorder_requests::list_reorder_requests)
                .post(inventory_reorder_requests::create_reorder_request),
        )
        .route(
            "/inventory-reorder-requests/:id",
            get(inventory_reorder_requests::get_reorder_request),
        )
        .route(
            "/inventory-reorder-requests/:id/submit",
            post(inventory_reorder_requests::submit_reorder_request),
        )
        .route(
            "/inventory-reorder-requests/:id/cancel",
            post(inventory_reorder_requests::cancel_reorder_request),
        )
        // Tax rules (jurisdiction → tax rate mapping)
        .route(
            "/tax-rules",
            get(tax_rules::list_tax_rules).post(tax_rules::create_tax_rule),
        )
        .route("/tax-rules/suggest", get(tax_rules::suggest_tax_rate))
        .route(
            "/tax-rules/:id",
            get(tax_rules::get_tax_rule)
                .patch(tax_rules::update_tax_rule)
                .delete(tax_rules::delete_tax_rule),
        )
        // Service territories (field service / work order dispatch)
        .route(
            "/service-territories",
            get(service_territories::list_service_territories)
                .post(service_territories::create_service_territory),
        )
        .route(
            "/service-territories/:id",
            get(service_territories::get_service_territory)
                .patch(service_territories::update_service_territory)
                .delete(service_territories::delete_service_territory),
        )
        // Subscription billing
        .route(
            "/subscriptions/:id/bill",
            post(subscriptions::bill_subscription),
        )
        .route(
            "/subscriptions/billing-run",
            post(subscriptions::billing_run),
        )
        // Payroll summary report
        .route("/reports/payroll-summary", get(reports::payroll_summary))
        // GRNI accrual report
        .route("/reports/grni-accrual", get(reports::grni_accrual))
        // AR / AP aging detail
        .route("/reports/ar-aging-detail", get(reports::ar_aging_detail))
        .route("/reports/ap-aging-detail", get(reports::ap_aging_detail))
        // Sales by customer
        .route(
            "/reports/sales-by-customer",
            get(reports::sales_by_customer),
        )
        .route(
            "/reports/outstanding-quotes",
            get(reports::outstanding_quotes),
        )
        .route("/reports/po-spending", get(reports::po_spending))
        .route(
            "/reports/cash-flow-indirect",
            get(reports::cash_flow_indirect),
        )
        .route("/reports/vat-return", get(reports::vat_return))
        .route(
            "/reports/sales-tax-by-nexus",
            get(reports::sales_tax_by_nexus),
        )
        .route(
            "/reports/currency-exposure",
            get(reports::currency_exposure),
        )
        .route(
            "/reports/cash-receipts-journal",
            get(reports::cash_receipts_journal),
        )
        .route(
            "/reports/cash-disbursements-journal",
            get(reports::cash_disbursements_journal),
        )
        .route(
            "/reports/pl-by-tracking-category",
            get(reports::pl_by_tracking_category),
        )
        .route("/reports/equity-statement", get(reports::equity_statement))
        .route("/reports/inventory-aging", get(reports::inventory_aging))
        .route(
            "/reports/customer-balances",
            get(reports::customer_balances),
        )
        .route("/reports/vendor-balances", get(reports::vendor_balances))
        .route("/reports/sales-by-rep", get(reports::sales_by_rep))
        // Inventory serial number tracking
        .route(
            "/inventory-serial-numbers",
            get(inventory_serial_numbers::list_serial_numbers)
                .post(inventory_serial_numbers::create_serial_number),
        )
        .route(
            "/inventory-serial-numbers/:id",
            get(inventory_serial_numbers::get_serial_number)
                .patch(inventory_serial_numbers::update_serial_number)
                .delete(inventory_serial_numbers::delete_serial_number),
        )
        // Inventory stocktakes
        .route(
            "/inventory-stocktakes",
            get(inventory_stocktakes::list_stocktakes).post(inventory_stocktakes::create_stocktake),
        )
        .route(
            "/inventory-stocktakes/:id",
            get(inventory_stocktakes::get_stocktake),
        )
        .route(
            "/inventory-stocktakes/:id/lines/:line_id",
            axum::routing::patch(inventory_stocktakes::update_stocktake_line),
        )
        .route(
            "/inventory-stocktakes/:id/submit",
            post(inventory_stocktakes::submit_stocktake),
        )
        .route(
            "/inventory-stocktakes/:id/post",
            post(inventory_stocktakes::post_stocktake),
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
