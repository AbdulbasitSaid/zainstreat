pub mod categories;
pub mod health;
pub mod menu_items;

use axum::{routing::get, Router};
use sqlx::PgPool;

pub fn api_router() -> Router<PgPool> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
}
