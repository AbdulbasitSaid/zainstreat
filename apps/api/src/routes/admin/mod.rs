pub mod auth;
pub mod categories;
pub mod delivery;
pub mod drafts;
pub mod enquiries;
pub mod events;
pub mod media;
pub mod menu_items;
pub mod middleware;
pub mod orders;
pub mod privacy;
pub mod push;
pub mod quotes;
pub mod rentals;
pub mod site_content;
pub mod translations;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use crate::AppState;
use axum::{
    Router,
    middleware::from_fn,
    routing::{delete, get, patch, post, put},
};

pub fn admin_router() -> Router<AppState> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/orders", get(orders::list_orders))
        .route("/orders/summary", get(orders::orders_summary))
        .route("/orders/{id}", get(orders::get_order))
        .route("/orders/{id}/status", patch(orders::update_order_status))
        // Phase 37: the delivery fee.
        .route("/orders/{id}/delivery-fee", put(delivery::set_order_fee))
        .route("/delivery-settings", get(delivery::get_settings).put(delivery::save_settings))
        .route("/delivery/suggest", get(delivery::suggest))
        .route("/delivery/quote", post(delivery::quote))
        // Phase 38: DeepL settings and usage.
        .route("/translations/status", get(translations::get_status))
        .route("/translations/settings", put(translations::save_settings))
        .route("/categories", get(categories::list_categories).post(categories::create_category))
        .route("/categories/{id}", get(categories::get_category))
        .route("/categories/{id}/draft", put(categories::save_category_draft))
        .route("/categories/{id}/archive", post(categories::archive_category))
        .route("/menu-items", get(menu_items::list_menu_items).post(menu_items::create_menu_item))
        .route("/menu-items/{id}", get(menu_items::get_menu_item))
        .route("/menu-items/{id}/draft", put(menu_items::save_menu_item_draft))
        .route("/menu-items/{id}/availability", patch(menu_items::update_availability))
        .route("/menu-items/{id}/archive", post(menu_items::archive_menu_item))
        .route("/rentals", get(rentals::list_rentals).post(rentals::create_rental))
        .route("/rentals/{id}", get(rentals::get_rental).put(rentals::update_rental))
        .route("/rentals/{id}/archive", post(rentals::archive_rental))
        .route("/media", media::upload_router())
        .route(
            "/catering-enquiries",
            get(enquiries::list_catering_enquiries),
        )
        .route(
            "/catering-enquiries/{id}",
            get(enquiries::get_catering_enquiry),
        )
        .route("/catering-enquiries/{id}/quote", put(quotes::save_quote))
        .route(
            "/catering-enquiries/{id}/quote/send",
            post(quotes::send_quote),
        )
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
        // Phase 31: events.
        .route("/events", get(events::list_events).post(events::create_event))
        .route("/events/{id}", get(events::get_event).put(events::update_event))
        .route("/events/{id}/publish", post(events::publish_event))
        .route("/events/{id}/unpublish", post(events::unpublish_event))
        .route("/events/{id}/archive", post(events::archive_event))
        .route("/events/{id}/unarchive", post(events::unarchive_event))
        .route("/events/{id}/photos", put(events::save_photos))
        .route("/events/{id}/bookings", get(events::list_bookings))
        .route("/events/{id}/bookings.csv", get(events::bookings_csv))
        // Phase 38: the attendee list once bookings close.
        .route("/events/{id}/attendees", get(events::list_attendees))
        .route("/events/{id}/close", post(events::close_event))
        .route("/events/{id}/reopen", post(events::reopen_event))
        .route("/events/{id}/attendees.csv", get(events::attendees_csv))
        .route("/events/bookings/{id}/offer", post(events::offer_seats))
        .route("/events/bookings/{id}/cancel", post(events::cancel_booking))
        .route("/preview/events/{slug}", get(events::preview_event))
        // Phase 27: admin web push.
        .route("/push/public-key", get(push::get_public_key))
        .route("/push/subscriptions", get(push::list_subscriptions).post(push::subscribe))
        .route("/push/subscriptions/{id}", delete(push::delete_subscription))
        .route("/push/test", post(push::send_test))
        // Phase 33: GDPR access and erasure requests.
        .route("/privacy", get(privacy::search))
        .route("/privacy/export", get(privacy::export))
        .route("/privacy/anonymise", post(privacy::anonymise))
        .route_layer(from_fn(middleware::require_admin));

    Router::new()
        .route(
            "/login",
            post(auth::login).route_layer(crate::rate_limit::login_rate_limiter()),
        )
        .merge(protected)
}
