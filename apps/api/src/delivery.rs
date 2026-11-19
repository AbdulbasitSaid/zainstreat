//! Phase 37: the delivery fee (requirement.md Decisions 5–12, 17).
//!
//! Addresses are looked up with PDOK Locatieserver (Dutch addresses, no
//! key) and the driving distance comes from the Mapbox Directions API. When
//! Mapbox is down, unconfigured or over quota, the distance falls back to
//! the straight line × 1.3 (Decision 10). The business origin is private:
//! nothing here logs an address or coordinates, and only the admin API
//! returns the origin (Decision 7).

use chrono::{DateTime, Utc};
use rust_decimal::prelude::{FromPrimitive, RoundingStrategy};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

const PDOK_BASE_URL: &str = "https://api.pdok.nl/bzk/locatieserver/search/v3_1";
const MAPBOX_DIRECTIONS_URL: &str = "https://api.mapbox.com/directions/v5/mapbox/driving";
const PDOK_TIMEOUT: Duration = Duration::from_secs(4);
const ROUTE_TIMEOUT: Duration = Duration::from_secs(5);
const SUGGESTION_ROWS: u32 = 7;
/// Roads are longer than the crow flies (Decision 10).
pub const STRAIGHT_LINE_FACTOR: f64 = 1.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimatedBy {
    Route,
    StraightLine,
    /// Set by the owner in admin (Decision 17).
    Manual,
}

impl EstimatedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::StraightLine => "straight_line",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedAddress {
    pub id: String,
    pub label: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone)]
pub struct DeliveryQuote {
    pub address_id: String,
    pub address: String,
    pub distance_km: Decimal,
    pub fee: Decimal,
    pub rate_per_km: Decimal,
    pub estimated_by: EstimatedBy,
}

impl DeliveryQuote {
    /// The distance an order may keep. Mapbox's terms forbid storing results
    /// from its Navigation APIs (Product Terms §2.10.1), so a routed
    /// distance is shown live at checkout but never saved; our own
    /// straight-line estimate may be.
    pub fn storable_distance_km(&self) -> Option<Decimal> {
        (self.estimated_by != EstimatedBy::Route).then_some(self.distance_km)
    }
}

#[derive(Debug)]
pub enum DeliveryError {
    /// Delivery is switched off, or there is no located business address.
    Unavailable,
    OutOfRange,
    NotFound,
    /// PDOK didn't answer (Decision 17).
    LookupDown,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for DeliveryError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

/// PDOK failed to answer, as opposed to answering "no match".
#[derive(Debug)]
pub struct LookupDown;

#[derive(Debug, Clone)]
pub struct DeliverySettings {
    pub origin_address: Option<String>,
    pub origin_lat: Option<f64>,
    pub origin_lon: Option<f64>,
    pub rate_per_km: Decimal,
    pub minimum_fee: Decimal,
    pub max_distance_km: Option<Decimal>,
    pub delivery_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl DeliverySettings {
    pub fn origin(&self) -> Option<(f64, f64)> {
        self.origin_lat.zip(self.origin_lon)
    }
}

pub async fn load_settings(pool: &PgPool) -> Result<DeliverySettings, sqlx::Error> {
    let mut settings = sqlx::query_as!(
        DeliverySettings,
        r#"
        SELECT origin_address, origin_lat, origin_lon, rate_per_km, minimum_fee,
               max_distance_km, delivery_enabled, updated_at as "updated_at: DateTime<Utc>"
        FROM delivery_settings
        "#
    )
    .fetch_one(pool)
    .await?;
    // A zero NUMERIC decodes without its scale; money always shows cents.
    settings.rate_per_km.rescale(2);
    settings.minimum_fee.rescale(2);
    Ok(settings)
}

/// One-way distance in km, rounded to one decimal (Decision 8).
pub fn round_km(km: f64) -> Decimal {
    Decimal::from_f64(km)
        .unwrap_or(Decimal::ZERO)
        .round_dp_with_strategy(1, RoundingStrategy::MidpointAwayFromZero)
}

/// `max(minimum_fee, distance × rate)`, in cents (Decision 8).
pub fn compute_fee(distance_km: Decimal, rate_per_km: Decimal, minimum_fee: Decimal) -> Decimal {
    let fee = (distance_km * rate_per_km).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    fee.max(minimum_fee)
}

pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let (lat1, lon1) = (a.0.to_radians(), a.1.to_radians());
    let (lat2, lon2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().asin()
}

/// PDOK's `centroide_ll` is WKT `POINT(lon lat)`; returns `(lat, lon)`.
pub fn parse_centroid(wkt: &str) -> Option<(f64, f64)> {
    let inner = wkt.trim().strip_prefix("POINT(")?.strip_suffix(')')?;
    let mut parts = inner.split_whitespace();
    let lon = parts.next()?.parse().ok()?;
    let lat = parts.next()?.parse().ok()?;
    Some((lat, lon))
}

/// Mapbox's `routes[0].distance` (metres) in km; `None` for an empty or odd response.
pub fn parse_mapbox_distance_km(body: &Value) -> Option<f64> {
    let meters = body.get("routes")?.get(0)?.get("distance")?.as_f64()?;
    Some(meters / 1000.0)
}

/// PDOK ids are short ASCII tokens such as `adr-…`; anything else is
/// rejected before it reaches a URL.
pub fn is_valid_address_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A canned backend for tests, so they never touch the network.
#[derive(Debug, Clone, Default)]
pub struct FakeBackend {
    pub addresses: Vec<(String, String, f64, f64)>,
    /// PDOK behaves as if down.
    pub lookup_down: bool,
    /// The routed distance; `None` behaves as if Mapbox failed.
    pub route_km: Option<f64>,
}

struct HttpBackend {
    client: reqwest::Client,
    pdok_base_url: String,
    mapbox_access_token: Option<String>,
}

#[derive(Clone)]
enum Backend {
    Http(Arc<HttpBackend>),
    Fake(Arc<FakeBackend>),
}

#[derive(Clone)]
pub struct DeliveryConfig {
    backend: Backend,
}

impl DeliveryConfig {
    /// `MAPBOX_ACCESS_TOKEN` unset means straight-line distances only.
    /// `PDOK_BASE_URL` exists so the lookup-down path can be tried locally.
    pub fn from_env() -> Self {
        let non_blank = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        let mapbox_access_token = non_blank("MAPBOX_ACCESS_TOKEN");
        if mapbox_access_token.is_none() {
            tracing::info!("MAPBOX_ACCESS_TOKEN not set: delivery distances use the straight-line estimate");
        }
        let client = reqwest::Client::builder()
            .timeout(ROUTE_TIMEOUT)
            .build()
            .expect("reqwest client builds");
        Self {
            backend: Backend::Http(Arc::new(HttpBackend {
                client,
                pdok_base_url: non_blank("PDOK_BASE_URL").unwrap_or_else(|| PDOK_BASE_URL.to_string()),
                mapbox_access_token,
            })),
        }
    }

    pub fn fake(fake: FakeBackend) -> Self {
        Self { backend: Backend::Fake(Arc::new(fake)) }
    }

    pub async fn suggest(&self, q: &str) -> Result<Vec<Suggestion>, LookupDown> {
        match &self.backend {
            Backend::Http(http) => http.suggest(q).await,
            Backend::Fake(fake) => {
                if fake.lookup_down {
                    return Err(LookupDown);
                }
                let q = q.to_lowercase();
                Ok(fake
                    .addresses
                    .iter()
                    .filter(|(_, label, _, _)| label.to_lowercase().contains(&q))
                    .map(|(id, label, _, _)| Suggestion { id: id.clone(), label: label.clone() })
                    .collect())
            }
        }
    }

    pub async fn lookup(&self, id: &str) -> Result<Option<ResolvedAddress>, LookupDown> {
        match &self.backend {
            Backend::Http(http) => http.lookup(id).await,
            Backend::Fake(fake) => {
                if fake.lookup_down {
                    return Err(LookupDown);
                }
                Ok(fake.addresses.iter().find(|(fid, ..)| fid == id).map(|(id, label, lat, lon)| {
                    ResolvedAddress { id: id.clone(), label: label.clone(), lat: *lat, lon: *lon }
                }))
            }
        }
    }

    /// The best address match for free text (the seeded origin, and the
    /// "is PDOK really down?" check of Decision 17).
    pub async fn geocode(&self, text: &str) -> Result<Option<ResolvedAddress>, LookupDown> {
        match &self.backend {
            Backend::Http(http) => http.geocode(text).await,
            Backend::Fake(fake) => {
                if fake.lookup_down {
                    return Err(LookupDown);
                }
                let text = text.to_lowercase();
                Ok(fake
                    .addresses
                    .iter()
                    .find(|(_, label, _, _)| text.contains(&label.to_lowercase()) || label.to_lowercase().contains(&text))
                    .map(|(id, label, lat, lon)| ResolvedAddress { id: id.clone(), label: label.clone(), lat: *lat, lon: *lon }))
            }
        }
    }

    async fn route_km(&self, from: (f64, f64), to: (f64, f64)) -> Option<f64> {
        match &self.backend {
            Backend::Http(http) => http.route_km(from, to).await,
            Backend::Fake(fake) => fake.route_km,
        }
    }

    /// Fills in the origin's coordinates when only its text is known (the
    /// migration seeds text only, Decision 6). `None` when there is no
    /// origin or it can't be located right now.
    pub async fn ensure_origin(
        &self,
        pool: &PgPool,
        settings: &mut DeliverySettings,
    ) -> Result<Option<(f64, f64)>, sqlx::Error> {
        if let Some(origin) = settings.origin() {
            return Ok(Some(origin));
        }
        let Some(text) = settings.origin_address.clone().filter(|t| !t.trim().is_empty()) else {
            return Ok(None);
        };
        let Ok(Some(found)) = self.geocode(&text).await else {
            return Ok(None);
        };
        // Only if nobody changed the address meanwhile; updated_at is left
        // alone so an open admin form doesn't conflict over this.
        sqlx::query!(
            "UPDATE delivery_settings SET origin_lat = $1, origin_lon = $2 \
             WHERE origin_address = $3 AND origin_lat IS NULL",
            found.lat,
            found.lon,
            text,
        )
        .execute(pool)
        .await?;
        settings.origin_lat = Some(found.lat);
        settings.origin_lon = Some(found.lon);
        Ok(Some((found.lat, found.lon)))
    }

    /// Distance and fee for a looked-up address. `require_enabled` is false
    /// for the owner's "Test an address", which must work before delivery is
    /// switched on (Decision 15).
    pub async fn quote(
        &self,
        pool: &PgPool,
        address_id: &str,
        require_enabled: bool,
    ) -> Result<DeliveryQuote, DeliveryError> {
        let mut settings = load_settings(pool).await?;
        if require_enabled && !settings.delivery_enabled {
            return Err(DeliveryError::Unavailable);
        }
        let destination = self
            .lookup(address_id)
            .await
            .map_err(|LookupDown| DeliveryError::LookupDown)?
            .ok_or(DeliveryError::NotFound)?;
        let Some(origin) = self.ensure_origin(pool, &mut settings).await? else {
            return Err(DeliveryError::Unavailable);
        };
        let to = (destination.lat, destination.lon);
        let (km, estimated_by) = match self.route_km(origin, to).await {
            Some(km) => (km, EstimatedBy::Route),
            None => (haversine_km(origin, to) * STRAIGHT_LINE_FACTOR, EstimatedBy::StraightLine),
        };
        let distance_km = round_km(km);
        if settings.max_distance_km.is_some_and(|max| distance_km > max) {
            return Err(DeliveryError::OutOfRange);
        }
        Ok(DeliveryQuote {
            address_id: destination.id,
            address: destination.label,
            distance_km,
            fee: compute_fee(distance_km, settings.rate_per_km, settings.minimum_fee),
            rate_per_km: settings.rate_per_km,
            estimated_by,
        })
    }
}

#[derive(Deserialize)]
struct PdokResponse {
    response: PdokDocs,
}

#[derive(Deserialize)]
struct PdokDocs {
    docs: Vec<PdokDoc>,
}

#[derive(Deserialize)]
struct PdokDoc {
    id: String,
    weergavenaam: String,
    centroide_ll: Option<String>,
}

impl PdokDoc {
    fn resolve(self) -> Option<ResolvedAddress> {
        let (lat, lon) = parse_centroid(self.centroide_ll.as_deref()?)?;
        Some(ResolvedAddress { id: self.id, label: self.weergavenaam, lat, lon })
    }
}

impl HttpBackend {
    async fn pdok(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Vec<PdokDoc>, LookupDown> {
        let mut url = reqwest::Url::parse(&format!("{}/{endpoint}", self.pdok_base_url)).map_err(|_| LookupDown)?;
        url.query_pairs_mut().extend_pairs(params);
        let response = self.client.get(url).timeout(PDOK_TIMEOUT).send().await.map_err(|error| {
            tracing::warn!(endpoint, timeout = error.is_timeout(), "PDOK request failed");
            LookupDown
        })?;
        let status = response.status();
        if !status.is_success() {
            tracing::warn!(endpoint, %status, "PDOK returned an error");
            return Err(LookupDown);
        }
        let bytes = response.bytes().await.map_err(|_| LookupDown)?;
        let parsed: PdokResponse = serde_json::from_slice(&bytes).map_err(|_| {
            tracing::warn!(endpoint, "PDOK response didn't parse");
            LookupDown
        })?;
        Ok(parsed.response.docs)
    }

    async fn suggest(&self, q: &str) -> Result<Vec<Suggestion>, LookupDown> {
        let rows = SUGGESTION_ROWS.to_string();
        let docs = self.pdok("suggest", &[("q", q), ("fq", "type:adres"), ("rows", &rows)]).await?;
        Ok(docs.into_iter().map(|doc| Suggestion { id: doc.id, label: doc.weergavenaam }).collect())
    }

    async fn lookup(&self, id: &str) -> Result<Option<ResolvedAddress>, LookupDown> {
        let docs = self.pdok("lookup", &[("id", id), ("fl", "id,weergavenaam,centroide_ll")]).await?;
        Ok(docs.into_iter().next().and_then(PdokDoc::resolve))
    }

    async fn geocode(&self, text: &str) -> Result<Option<ResolvedAddress>, LookupDown> {
        let docs = self
            .pdok("free", &[("q", text), ("fq", "type:adres"), ("rows", "1"), ("fl", "id,weergavenaam,centroide_ll")])
            .await?;
        Ok(docs.into_iter().next().and_then(PdokDoc::resolve))
    }

    async fn route_km(&self, from: (f64, f64), to: (f64, f64)) -> Option<f64> {
        let token = self.mapbox_access_token.as_deref()?;
        // `mapbox/driving` (no live traffic): traffic has no place in a fixed
        // fee (Decision 10). Only coordinates leave the server, as lon,lat.
        let mut url = reqwest::Url::parse(&format!(
            "{MAPBOX_DIRECTIONS_URL}/{},{};{},{}",
            from.1, from.0, to.1, to.0
        ))
        .ok()?;
        url.query_pairs_mut()
            .append_pair("alternatives", "false")
            .append_pair("overview", "false")
            .append_pair("steps", "false")
            .append_pair("access_token", token);
        // Never log the error itself: its URL carries the token and the
        // customer's coordinates.
        let response = self
            .client
            .get(url)
            .send()
            .await
            .inspect_err(|error| tracing::warn!(timeout = error.is_timeout(), "Mapbox Directions request failed"))
            .ok()?;
        let status = response.status();
        if !status.is_success() {
            tracing::warn!(%status, "Mapbox Directions returned an error; using the straight-line estimate");
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        let parsed: Value = serde_json::from_slice(&bytes).ok()?;
        let km = parse_mapbox_distance_km(&parsed);
        if km.is_none() {
            tracing::warn!("Mapbox Directions found no route; using the straight-line estimate");
        }
        km
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dec(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    #[test]
    fn fee_uses_rounded_km_times_rate() {
        assert_eq!(round_km(12.34), dec("12.3"));
        assert_eq!(compute_fee(dec("12.3"), dec("1.00"), Decimal::ZERO), dec("12.30"));
        assert_eq!(compute_fee(dec("7.5"), dec("1.50"), Decimal::ZERO), dec("11.25"));
    }

    #[test]
    fn fee_never_below_minimum() {
        assert_eq!(compute_fee(dec("2.0"), dec("1.00"), dec("5.00")), dec("5.00"));
        assert_eq!(compute_fee(dec("12.0"), Decimal::ZERO, Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn km_rounds_half_away_from_zero() {
        assert_eq!(round_km(0.05), dec("0.1"));
        assert_eq!(round_km(24.449), dec("24.4"));
    }

    #[test]
    fn haversine_amsterdam_to_almere() {
        // Amsterdam Centraal → Almere Centrum is about 24 km as the crow flies.
        let km = haversine_km((52.3791, 4.9003), (52.3702, 5.2141));
        assert!((20.0..=25.0).contains(&km), "{km}");
    }

    #[test]
    fn parses_pdok_centroid() {
        assert_eq!(parse_centroid("POINT(5.21 52.37)"), Some((52.37, 5.21)));
        assert_eq!(parse_centroid("POLYGON(1 2)"), None);
        assert_eq!(parse_centroid("POINT(x y)"), None);
    }

    #[test]
    fn parses_mapbox_distance() {
        assert_eq!(parse_mapbox_distance_km(&json!({ "code": "Ok", "routes": [{ "distance": 12345.0 }] })), Some(12.345));
        assert_eq!(parse_mapbox_distance_km(&json!({ "code": "NoRoute", "routes": [] })), None);
        assert_eq!(parse_mapbox_distance_km(&json!({})), None);
    }

    #[test]
    fn address_ids_are_plain_tokens() {
        assert!(is_valid_address_id("adr-5c4b1a2f0d3e6f7a8b9c0d1e2f3a4b5c"));
        assert!(!is_valid_address_id(""));
        assert!(!is_valid_address_id("adr/../x"));
        assert!(!is_valid_address_id("a b"));
    }
}
