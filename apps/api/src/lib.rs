pub mod error;
pub mod routes;

use axum::Router;
use sqlx::PgPool;

pub fn build_app(pool: PgPool) -> Router {
    Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health))
        .with_state(pool)
}
