pub mod auth;
pub mod middleware;

pub(crate) const SESSION_USER_ID_KEY: &str = "user_id";

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub fn admin_router() -> Router<PgPool> {
    let protected = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route_layer(from_fn(middleware::require_admin));

    Router::new().route("/login", post(auth::login)).merge(protected)
}
