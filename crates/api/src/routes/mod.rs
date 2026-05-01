use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{
    handlers::{accounts, auth, contacts, invoices, transactions},
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let public = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login));

    let protected = Router::new()
        // Accounts (Chart of Accounts)
        .route("/accounts", get(accounts::list_accounts).post(accounts::create_account))
        .route(
            "/accounts/:id",
            get(accounts::get_account)
                .patch(accounts::update_account)
                .delete(accounts::delete_account),
        )
        // Journal entries
        .route(
            "/transactions",
            get(transactions::list_transactions).post(transactions::create_transaction),
        )
        .route("/transactions/:id", get(transactions::get_transaction))
        // Contacts
        .route("/contacts", get(contacts::list_contacts).post(contacts::create_contact))
        .route(
            "/contacts/:id",
            get(contacts::get_contact).patch(contacts::update_contact),
        )
        // Invoices & bills
        .route("/invoices", get(invoices::list_invoices).post(invoices::create_invoice))
        .route("/invoices/:id", get(invoices::get_invoice))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .nest("/api/v1", public.merge(protected))
        .route("/health", get(health_check))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "ok"
}
