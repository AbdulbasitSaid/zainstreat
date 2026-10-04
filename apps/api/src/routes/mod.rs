pub mod admin;
pub mod categories;
pub mod health;
pub mod media;
pub mod menu_items;
pub mod orders;

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(categories::list_categories))
        .route("/menu-items", get(menu_items::list_menu_items))
        .route("/orders", post(orders::create_order))
        .route("/media/{key}", get(media::get_media))
        .nest("/admin", admin::admin_router())
}
