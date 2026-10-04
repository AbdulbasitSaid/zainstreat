pub mod admin;
pub mod catering_enquiries;
pub mod categories;
pub mod contact_messages;
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
        .route(
            "/catering-enquiries",
            post(catering_enquiries::create_catering_enquiry)
                .route_layer(crate::rate_limit::enquiry_rate_limiter()),
        )
        .route(
            "/contact-messages",
            post(contact_messages::create_contact_message)
                .route_layer(crate::rate_limit::enquiry_rate_limiter()),
        )
        .route("/media/{key}", get(media::get_media))
        .nest("/admin", admin::admin_router())
}
