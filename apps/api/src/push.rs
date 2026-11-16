//! Web push to the owner's devices (Phase 27). Standard Web Push (RFC 8030)
//! with VAPID (RFC 8292) and `aes128gcm` (RFC 8291) via `web-push-native`,
//! sent with `reqwest`. Fire-and-forget like email (Decision 7): nothing here
//! ever fails or delays the request that triggered it.

use crate::routes::orders::OrderResponse;
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::NaiveDate;
use reqwest::header::HeaderValue;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use web_push_native::jwt_simple::algorithms::{ECDSAP256PublicKeyLike, ES256KeyPair};
use web_push_native::{Auth, WebPushBuilder, p256::PublicKey};

/// How long the push service keeps a message for an offline device
/// (Decision 14).
const PUSH_TTL_SECONDS: &str = "86400";
/// The VAPID token's lifetime. Kept apart from the TTL above because Apple
/// rejects tokens that expire more than 24 hours out.
const VAPID_TOKEN_LIFETIME: Duration = Duration::from_secs(12 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const BODY_MAX_CHARS: usize = 200;
const CONTACT_PREVIEW_CHARS: usize = 100;

#[derive(Clone)]
pub struct PushConfig {
    settings: Option<Arc<PushSettings>>,
}

struct PushSettings {
    public_key: String,
    key_pair: ES256KeyPair,
    subject: String,
    client: reqwest::Client,
}

/// One notification for the owner. `url` is an admin path, never a full URL,
/// so a push can't send the app elsewhere (Decision 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerPush {
    pub title: String,
    pub body: String,
    pub url: String,
    pub tag: String,
}

#[derive(Serialize)]
struct Payload<'a> {
    title: &'a str,
    body: &'a str,
    url: &'a str,
    tag: &'a str,
    /// Orders needing action, for the app icon badge (Decision 11).
    #[serde(skip_serializing_if = "Option::is_none")]
    badge: Option<i64>,
}

struct Subscription {
    id: i64,
    endpoint: String,
    p256dh: String,
    auth: String,
}

enum Outcome {
    Delivered,
    /// 404/410: the browser dropped the subscription (Decision 10).
    Gone,
    Failed(String),
}

impl PushConfig {
    pub fn disabled() -> Self {
        Self { settings: None }
    }

    /// Off (logged once) when any of the three variables is unset or blank,
    /// or when the keys don't parse or don't match (Decision 16).
    pub fn from_env() -> Self {
        let non_blank = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        let (Some(public_key), Some(private_key), Some(subject)) = (
            non_blank("VAPID_PUBLIC_KEY"),
            non_blank("VAPID_PRIVATE_KEY"),
            non_blank("VAPID_SUBJECT"),
        ) else {
            tracing::info!("web push disabled: VAPID env vars not fully set");
            return Self::disabled();
        };
        match Self::from_keys(&public_key, &private_key, &subject) {
            Ok(config) => config,
            Err(error) => {
                tracing::error!(%error, "web push disabled: invalid VAPID keys");
                Self::disabled()
            }
        }
    }

    pub fn from_keys(public_key: &str, private_key: &str, subject: &str) -> Result<Self, String> {
        let private_bytes = Base64UrlUnpadded::decode_vec(private_key.trim())
            .map_err(|_| "VAPID_PRIVATE_KEY is not unpadded base64url".to_string())?;
        let key_pair = ES256KeyPair::from_bytes(&private_bytes).map_err(|e| e.to_string())?;
        // Browsers subscribe with the public key; a pair that doesn't match
        // would fail every send with a 403, so refuse it up front.
        let derived = encode_public_key(&key_pair);
        if derived != public_key.trim() {
            return Err("VAPID_PUBLIC_KEY doesn't match VAPID_PRIVATE_KEY".to_string());
        }
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            settings: Some(Arc::new(PushSettings {
                public_key: derived,
                key_pair,
                subject: subject.trim().to_string(),
                client,
            })),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.settings.is_some()
    }

    /// The browser's `applicationServerKey`.
    pub fn public_key(&self) -> Option<&str> {
        self.settings.as_ref().map(|s| s.public_key.as_str())
    }

    /// Every admin's devices.
    pub fn spawn_notify_owner(&self, pool: &PgPool, push: OwnerPush) {
        self.spawn(pool, None, push);
    }

    /// One admin's devices (the Settings test button).
    pub fn spawn_notify_user(&self, pool: &PgPool, user_id: i64, push: OwnerPush) {
        self.spawn(pool, Some(user_id), push);
    }

    fn spawn(&self, pool: &PgPool, user_id: Option<i64>, push: OwnerPush) {
        let Some(settings) = self.settings.clone() else {
            tracing::debug!(tag = push.tag, "web push disabled, skipping");
            return;
        };
        let pool = pool.clone();
        tokio::spawn(async move { deliver(settings, pool, user_id, push).await });
    }
}

/// `VAPID_PUBLIC_KEY` for a key pair: the uncompressed P-256 point,
/// unpadded base64url.
pub fn encode_public_key(key_pair: &ES256KeyPair) -> String {
    Base64UrlUnpadded::encode_string(&key_pair.public_key().public_key().to_bytes_uncompressed())
}

/// `VAPID_PRIVATE_KEY` for a key pair: the 32-byte scalar, unpadded
/// base64url.
pub fn encode_private_key(key_pair: &ES256KeyPair) -> String {
    Base64UrlUnpadded::encode_string(&key_pair.to_bytes())
}

async fn deliver(settings: Arc<PushSettings>, pool: PgPool, user_id: Option<i64>, push: OwnerPush) {
    let badge = match sqlx::query_scalar!(
        "SELECT COUNT(*) FROM orders WHERE status IN ('awaiting_payment', 'payment_claimed')"
    )
    .fetch_one(&pool)
    .await
    {
        Ok(count) => count,
        Err(error) => {
            tracing::warn!(%error, "push badge count failed");
            None
        }
    };

    let subscriptions = match sqlx::query_as!(
        Subscription,
        r#"
        SELECT id, endpoint, p256dh, auth FROM push_subscriptions
        WHERE $1::BIGINT IS NULL OR user_id = $1
        "#,
        user_id,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "failed to load push subscriptions");
            return;
        }
    };
    if subscriptions.is_empty() {
        tracing::debug!(tag = push.tag, "no push subscriptions");
        return;
    }

    let body = truncate_chars(&push.body, BODY_MAX_CHARS);
    let payload: Arc<[u8]> = match serde_json::to_vec(&Payload {
        title: &push.title,
        body: &body,
        url: &push.url,
        tag: &push.tag,
        badge,
    }) {
        Ok(bytes) => bytes.into(),
        Err(error) => {
            tracing::error!(%error, "failed to serialize push payload");
            return;
        }
    };

    let mut sends = JoinSet::new();
    for subscription in subscriptions {
        let settings = settings.clone();
        let pool = pool.clone();
        let payload = payload.clone();
        sends.spawn(async move {
            let outcome = send_one(&settings, &subscription, &payload).await;
            record_outcome(&pool, &subscription, outcome).await;
        });
    }
    while sends.join_next().await.is_some() {}
}

async fn send_one(settings: &PushSettings, subscription: &Subscription, payload: &[u8]) -> Outcome {
    let request = match build_request(settings, subscription, payload) {
        Ok(request) => request,
        Err(error) => return Outcome::Failed(error),
    };
    match settings.client.execute(request).await {
        Ok(response) => match response.status().as_u16() {
            200..=299 => Outcome::Delivered,
            404 | 410 => Outcome::Gone,
            status => Outcome::Failed(format!("push service answered {status}")),
        },
        Err(error) => Outcome::Failed(error.to_string()),
    }
}

fn build_request(
    settings: &PushSettings,
    subscription: &Subscription,
    payload: &[u8],
) -> Result<reqwest::Request, String> {
    let p256dh = Base64UrlUnpadded::decode_vec(&subscription.p256dh).map_err(|_| "bad p256dh".to_string())?;
    let auth = Base64UrlUnpadded::decode_vec(&subscription.auth).map_err(|_| "bad auth".to_string())?;
    if auth.len() != 16 {
        return Err("auth secret isn't 16 bytes".to_string());
    }
    let ua_public = PublicKey::from_sec1_bytes(&p256dh).map_err(|_| "bad p256dh point".to_string())?;
    let endpoint = subscription.endpoint.parse().map_err(|_| "bad endpoint".to_string())?;

    let http_request = WebPushBuilder::new(endpoint, ua_public, Auth::clone_from_slice(&auth))
        .with_valid_duration(VAPID_TOKEN_LIFETIME)
        .with_vapid(&settings.key_pair, &settings.subject)
        .build(payload.to_vec())
        .map_err(|e| e.to_string())?;
    let mut request = reqwest::Request::try_from(http_request).map_err(|e| e.to_string())?;
    let headers = request.headers_mut();
    headers.insert("ttl", HeaderValue::from_static(PUSH_TTL_SECONDS));
    headers.insert("urgency", HeaderValue::from_static("high"));
    Ok(request)
}

async fn record_outcome(pool: &PgPool, subscription: &Subscription, outcome: Outcome) {
    let result = match outcome {
        Outcome::Delivered => sqlx::query!(
            "UPDATE push_subscriptions SET last_success_at = now(), failure_count = 0 WHERE id = $1",
            subscription.id,
        )
        .execute(pool)
        .await
        .map(|_| ()),
        Outcome::Gone => {
            tracing::info!(subscription_id = subscription.id, "push subscription gone, removing");
            sqlx::query!("DELETE FROM push_subscriptions WHERE id = $1", subscription.id)
                .execute(pool)
                .await
                .map(|_| ())
        }
        Outcome::Failed(error) => {
            // The endpoint works as a credential, so only its host is logged.
            tracing::warn!(
                subscription_id = subscription.id,
                host = endpoint_host(&subscription.endpoint),
                %error,
                "push send failed"
            );
            sqlx::query!(
                r#"
                UPDATE push_subscriptions
                SET last_failure_at = now(), failure_count = failure_count + 1
                WHERE id = $1
                "#,
                subscription.id,
            )
            .execute(pool)
            .await
            .map(|_| ())
        }
    };
    if let Err(error) = result {
        tracing::error!(%error, subscription_id = subscription.id, "failed to record push outcome");
    }
}

fn endpoint_host(endpoint: &str) -> String {
    reqwest::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// A same-origin admin path (Decision 13): `/orders/1`, never `//host` or a
/// full URL.
pub fn is_admin_path(url: &str) -> bool {
    url.starts_with('/') && !url.starts_with("//")
}

// Message builders (Decision 6). English only, like the admin.

fn order_push(title: String, body: String, order_id: i64) -> OwnerPush {
    OwnerPush {
        title,
        body,
        url: format!("/orders/{order_id}"),
        tag: format!("order-{order_id}"),
    }
}

pub fn new_order(order: &OrderResponse) -> OwnerPush {
    let fulfilment = if order.delivery_type == "delivery" { "Delivery" } else { "Pickup" };
    order_push(
        format!("New order {}", order.payment_reference),
        format!("€{} from {} · {fulfilment}", order.total, order.customer_name),
        order.id,
    )
}

pub fn quote_accepted(quote_id: i64, order: &OrderResponse) -> OwnerPush {
    order_push(
        format!("Quote #{quote_id} accepted"),
        format!("Order {} · €{} from {}", order.payment_reference, order.total, order.customer_name),
        order.id,
    )
}

pub fn payment_claimed(order: &OrderResponse) -> OwnerPush {
    order_push(
        format!("Payment claimed — {}", order.payment_reference),
        format!("{} says they've paid €{}", order.customer_name, order.total),
        order.id,
    )
}

pub fn catering_enquiry(
    id: i64,
    name: &str,
    event_type: &str,
    event_date: NaiveDate,
    guest_count: i32,
) -> OwnerPush {
    OwnerPush {
        title: "New catering enquiry".to_string(),
        body: format!(
            "{name} · {event_type} · {} · {guest_count} guests",
            event_date.format("%d-%m-%Y")
        ),
        url: format!("/enquiries/catering/{id}"),
        tag: format!("enquiry-{id}"),
    }
}

pub fn quote_declined(quote_id: i64, enquiry_id: i64, name: &str, reason: Option<&str>) -> OwnerPush {
    let reason = reason.map(str::trim).filter(|r| !r.is_empty()).unwrap_or("No reason given");
    OwnerPush {
        title: format!("Quote #{quote_id} declined"),
        body: format!("{name} · {reason}"),
        url: format!("/enquiries/catering/{enquiry_id}"),
        tag: format!("enquiry-{enquiry_id}"),
    }
}

pub fn contact_message(id: i64, name: &str, subject: &str, message: &str) -> OwnerPush {
    OwnerPush {
        title: format!("New message from {name}"),
        body: format!("{subject}: {}", truncate_chars(message.trim(), CONTACT_PREVIEW_CHARS)),
        url: format!("/enquiries/contact/{id}"),
        tag: format!("contact-{id}"),
    }
}

pub fn test_notification() -> OwnerPush {
    OwnerPush {
        title: "Test notification".to_string(),
        body: "Notifications are working on this device.".to_string(),
        url: "/settings".to_string(),
        tag: "test".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_chars("héllo", 10), "héllo");
        assert_eq!(truncate_chars("héllo wörld", 5), "héllo…");
    }

    #[test]
    fn admin_path_check() {
        assert!(is_admin_path("/orders/1"));
        assert!(!is_admin_path("//evil.example/orders"));
        assert!(!is_admin_path("https://evil.example/"));
        assert!(!is_admin_path("orders/1"));
    }

    #[test]
    fn enquiry_and_contact_builders() {
        let date = NaiveDate::from_ymd_opt(2026, 12, 12).unwrap();
        let push = catering_enquiry(7, "Amina", "Wedding", date, 80);
        assert_eq!(push.title, "New catering enquiry");
        assert_eq!(push.body, "Amina · Wedding · 12-12-2026 · 80 guests");
        assert_eq!(push.url, "/enquiries/catering/7");
        assert_eq!(push.tag, "enquiry-7");

        let push = quote_declined(12, 7, "Amina", Some("  "));
        assert_eq!(push.title, "Quote #12 declined");
        assert_eq!(push.body, "Amina · No reason given");
        assert_eq!(push.url, "/enquiries/catering/7");
        assert_eq!(push.tag, "enquiry-7");

        let long = "x".repeat(150);
        let push = contact_message(3, "Amina", "Catering", &long);
        assert_eq!(push.title, "New message from Amina");
        assert_eq!(push.body, format!("Catering: {}…", "x".repeat(100)));
        assert_eq!(push.url, "/enquiries/contact/3");
        assert_eq!(push.tag, "contact-3");
    }

    #[test]
    fn every_builder_links_inside_the_admin() {
        let date = NaiveDate::from_ymd_opt(2026, 12, 12).unwrap();
        for push in [
            catering_enquiry(1, "A", "B", date, 1),
            quote_declined(1, 1, "A", None),
            contact_message(1, "A", "B", "C"),
            test_notification(),
        ] {
            assert!(is_admin_path(&push.url), "{}", push.url);
        }
    }

    #[test]
    fn key_pair_round_trips_and_mismatch_is_rejected() {
        let key_pair = ES256KeyPair::generate();
        let public = encode_public_key(&key_pair);
        let private = encode_private_key(&key_pair);
        assert_eq!(public.len(), 87);
        assert_eq!(private.len(), 43);

        let config = PushConfig::from_keys(&public, &private, "mailto:contact@zainstreat.com").unwrap();
        assert!(config.is_enabled());
        assert_eq!(config.public_key(), Some(public.as_str()));

        let other = encode_public_key(&ES256KeyPair::generate());
        assert!(PushConfig::from_keys(&other, &private, "mailto:x@example.com").is_err());
        assert!(PushConfig::from_keys(&public, "not base64!", "mailto:x@example.com").is_err());
    }

    #[test]
    fn disabled_config_has_no_key() {
        let config = PushConfig::disabled();
        assert!(!config.is_enabled());
        assert_eq!(config.public_key(), None);
    }
}
