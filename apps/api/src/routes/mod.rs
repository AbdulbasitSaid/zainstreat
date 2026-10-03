pub mod categories;
pub mod health;
pub mod menu_items;
pub mod orders;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn api_router() -> Router<PgPool> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
}
