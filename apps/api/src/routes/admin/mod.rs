pub mod auth;
pub mod categories;
pub mod media;
pub mod menu_items;
pub mod middleware;
pub mod orders;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use crate::AppState;
use axum::{
    middleware::from_fn,
    routing::{get, patch, post},
    Router,
};

pub fn admin_router() -> Router<AppState> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/orders", get(orders::list_orders))
        .route("/orders/summary", get(orders::orders_summary))
        .route("/orders/{id}", get(orders::get_order))
        .route("/orders/{id}/status", patch(orders::update_order_status))
        .route("/categories", get(categories::list_categories).post(categories::create_category))
        .route("/categories/{id}", patch(categories::update_category))
        .route("/categories/{id}/archive", post(categories::archive_category))
        .route("/menu-items", get(menu_items::list_menu_items).post(menu_items::create_menu_item))
        .route(
            "/menu-items/{id}",
            get(menu_items::get_menu_item).patch(menu_items::update_menu_item),
        )
        .route("/menu-items/{id}/availability", patch(menu_items::update_availability))
        .route("/menu-items/{id}/archive", post(menu_items::archive_menu_item))
        .route("/media", media::upload_router())
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
