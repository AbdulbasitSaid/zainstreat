use std::time::Duration;

use axum::{body::Body, http::Request};
use governor::middleware::NoOpMiddleware;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::KeyExtractor, GovernorError, GovernorLayer,
};

/// This app's browser traffic reaches `apps/api` only through `apps/web`'s
/// same-origin Route Handler proxy — a server-to-server `fetch` over the
/// Docker-internal network. The TCP peer `apps/api` sees for every request on
/// that path is the `web` container itself, not the visitor, so tower_governor's
/// built-in peer-IP (or "smart IP" guessing from headers it didn't ask for)
/// extractors would put every visitor in the same bucket. Instead: the web
/// proxies forward the real client IP (already on their own incoming
/// request's `X-Forwarded-For`, set by Caddy) straight through to `apps/api`
/// in the same header, and this extractor reads it.
#[derive(Clone)]
pub struct ForwardedForKeyExtractor;

impl KeyExtractor for ForwardedForKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        Ok(client_ip_key(req))
    }
}

pub fn client_ip_key<T>(req: &Request<T>) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        // Caddy appends (never replaces) an incoming X-Forwarded-For, so
        // the *last* entry is always the hop Caddy itself observed —
        // trusting the first entry would let a client spoof its own key.
        .and_then(|value| value.rsplit(',').next())
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        // No header at all (local dev, no Caddy in front): one shared
        // bucket rather than wiring up `ConnectInfo` for a peer-IP
        // fallback that only matters where this app isn't actually
        // deployed — every real deployment sits behind Caddy.
        .unwrap_or_else(|| "unknown".to_string())
}

const CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

/// Two independent calls (one per route, Group 7) build two independent
/// `GovernorLayer`s — each with its own bucket per client IP, not a combined
/// budget across both endpoints.
pub fn enquiry_rate_limiter() -> GovernorLayer<ForwardedForKeyExtractor, NoOpMiddleware, Body> {
    let config = GovernorConfigBuilder::default()
        .key_extractor(ForwardedForKeyExtractor)
        .per_second(30) // refill one token every 30s...
        .burst_size(5) // ...allowing short bursts up to 5
        .finish()
        .expect("rate limiter config is valid");

    // Keyed rate limiters never forget a key on their own — bound memory use
    // by periodically dropping keys whose state is indistinguishable from
    // fresh (governor crate's own recommendation for keyed limiters).
    let limiter = config.limiter().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            interval.tick().await;
            limiter.retain_recent();
            limiter.shrink_to_fit();
        }
    });

    GovernorLayer::new(config)
}
