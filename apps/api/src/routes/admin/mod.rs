pub mod auth;
pub mod categories;
pub mod drafts;
pub mod enquiries;
pub mod media;
pub mod menu_items;
pub mod middleware;
pub mod orders;
pub mod quotes;
pub mod site_content;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use crate::AppState;
use axum::{
    middleware::from_fn,
    routing::{delete, get, patch, post, put},
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
        .route("/categories/{id}", get(categories::get_category))
        .route("/categories/{id}/draft", put(categories::save_category_draft))
        .route("/categories/{id}/archive", post(categories::archive_category))
        .route("/menu-items", get(menu_items::list_menu_items).post(menu_items::create_menu_item))
        .route("/menu-items/{id}", get(menu_items::get_menu_item))
        .route("/menu-items/{id}/draft", put(menu_items::save_menu_item_draft))
        .route("/menu-items/{id}/availability", patch(menu_items::update_availability))
        .route("/menu-items/{id}/archive", post(menu_items::archive_menu_item))
        .route("/media", media::upload_router())
        .route("/catering-enquiries", get(enquiries::list_catering_enquiries))
        .route("/catering-enquiries/{id}", get(enquiries::get_catering_enquiry))
        .route("/catering-enquiries/{id}/quote", put(quotes::save_quote))
        .route("/catering-enquiries/{id}/quote/send", post(quotes::send_quote))
        .route("/contact-messages", get(enquiries::list_contact_messages))
        .route("/contact-messages/{id}", get(enquiries::get_contact_message))
        // Phase 25: drafts, live preview and publish.
        .route("/site-content", get(site_content::get_site_content))
        .route("/site-content/draft", put(site_content::save_site_content_draft))
        .route("/preview/menu-items", get(drafts::preview_menu_items))
        .route("/preview/categories", get(drafts::preview_categories))
        .route("/preview/site-content", get(drafts::preview_site_content))
        .route("/changes", get(drafts::list_changes))
        .route("/publish", post(drafts::publish))
        .route("/drafts/site_content", delete(drafts::discard_site_content_draft))
        .route("/drafts/{kind}/{id}", delete(drafts::discard_draft))
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
