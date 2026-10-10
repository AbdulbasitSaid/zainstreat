pub mod email;
pub mod error;
pub mod rate_limit;
pub mod routes;
pub mod validation;

use axum::extract::FromRef;
use axum::Router;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, SessionManagerLayer,
};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::Level;

#[derive(Clone)]
pub struct MediaConfig {
    pub client: aws_sdk_s3::Client,
    pub bucket: String,
    pub public_base_url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub media: MediaConfig,
    pub email: email::EmailConfig,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}

impl FromRef<AppState> for MediaConfig {
    fn from_ref(state: &AppState) -> MediaConfig {
        state.media.clone()
    }
}

impl FromRef<AppState> for email::EmailConfig {
    fn from_ref(state: &AppState) -> email::EmailConfig {
        state.email.clone()
    }
}

/// `tower-sessions-sqlx-store` pulls in sqlx's `time` feature alongside this
/// crate's own `chrono` feature; with both active, sqlx's compile-time query
/// macros stop accepting `chrono::DateTime<Utc>` as a *bind parameter* type
/// for `timestamptz` columns (only `time::OffsetDateTime` is registered) even
/// though `chrono::DateTime<Utc>` still decodes query *results* fine via an
/// explicit `as "col: DateTime<Utc>"` override. This converts an
/// optimistic-concurrency `updated_at` token for binding only — every other
/// type in this codebase stays chrono.
pub(crate) fn chrono_to_offset(dt: chrono::DateTime<chrono::Utc>) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp_nanos(
        dt.timestamp_nanos_opt().expect("updated_at fits in i64 nanoseconds") as i128,
    )
    .expect("valid offset datetime")
}

/// Same ambiguity as `chrono_to_offset`, for `DATE` columns:
/// `catering_enquiries.event_date` binds as a query parameter, which needs
/// `time::Date` even though the result still decodes back to
/// `chrono::NaiveDate` fine via an explicit `as "col: NaiveDate"` override.
pub(crate) fn chrono_date_to_offset(date: chrono::NaiveDate) -> time::Date {
    use chrono::Datelike;
    time::Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).expect("valid month"),
        date.day() as u8,
    )
    .expect("valid calendar date")
}

pub async fn build_app(
    pool: PgPool,
    cookie_secure: bool,
    media: MediaConfig,
    email: email::EmailConfig,
) -> Result<Router, sqlx::Error> {
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cookie_secure)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    Ok(Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health))
        .layer(session_layer)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .latency_unit(LatencyUnit::Millis),
                        ),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
        .with_state(AppState { pool, media, email }))
}
