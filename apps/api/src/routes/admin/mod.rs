pub mod auth;
pub mod middleware;
pub mod orders;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use axum::{
    middleware::from_fn,
    routing::{get, patch, post},
    Router,
};
use sqlx::PgPool;

pub fn admin_router() -> Router<PgPool> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/orders", get(orders::list_orders))
        .route("/orders/summary", get(orders::orders_summary))
        .route("/orders/{id}", get(orders::get_order))
        .route("/orders/{id}/status", patch(orders::update_order_status))
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
