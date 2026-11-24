//! Phase 38: Dutch and French menu text from the DeepL API (requirement.md
//! Decisions 1–7, 20).
//!
//! English stays in each table's own columns; nl/fr live in a
//! `translations` JSONB next to them:
//!
//! ```json
//! {"nl": {"name": "…", "description": "…", "source": {"name": "…", "description": "…"},
//!         "keep": 1, "manual": false, "pending": {"source": {…}, "attempts": 1}}}
//! ```
//!
//! `source` is the English the text was made from and `keep` the keep-list
//! version, so a stale translation is detectable without rewriting rows.
//! `pending` counts failed attempts on the current English; after
//! `MAX_ATTEMPTS` the row is parked until the English changes or the owner
//! presses Re-translate. Every DeepL call goes through `reserve`, which
//! refuses past the owner's stop line on DeepL's own usage numbers
//! (Decision 20). Only menu text is ever sent, and nothing here logs it.

use crate::order_emails::Locale;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// The locales DeepL fills; English is the source.
pub const TARGETS: [Locale; 2] = [Locale::Nl, Locale::Fr];
/// Failed attempts on the same English before a row is parked (Decision 20).
pub const MAX_ATTEMPTS: u64 = 3;
/// Most characters one save may send per language (Decision 20).
pub const SAVE_CHAR_CAP: u64 = 5_000;
/// Most characters one sweep run may send in total (Decision 20).
pub const SWEEP_CHAR_CAP: u64 = 20_000;
const USAGE_TTL: Duration = Duration::from_secs(600);
const DEEPL_TIMEOUT: Duration = Duration::from_secs(5);

/// One locale's state, as the admin forms and lists show it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Missing,
    Stale,
    Manual,
    ManualStale,
    Parked,
}

impl Status {
    pub fn needs_translation(self) -> bool {
        matches!(self, Self::Missing | Self::Stale)
    }

    /// For combining an item with its price options: the one the owner most
    /// needs to see wins.
    fn rank(self) -> u8 {
        match self {
            Self::Parked => 5,
            Self::Missing => 4,
            Self::Stale => 3,
            Self::ManualStale => 2,
            Self::Manual => 1,
            Self::Ok => 0,
        }
    }

    pub fn worst(statuses: impl IntoIterator<Item = Status>) -> Status {
        statuses.into_iter().max_by_key(|status| status.rank()).unwrap_or(Status::Ok)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatusPair {
    pub nl: Status,
    pub fr: Status,
}

impl StatusPair {
    pub fn of(translations: &Value, english: &Value, keep_version: i32) -> Self {
        Self {
            nl: status(translations, Locale::Nl, english, keep_version),
            fr: status(translations, Locale::Fr, english, keep_version),
        }
    }

    pub fn worst(pairs: impl IntoIterator<Item = StatusPair>) -> Self {
        let pairs: Vec<StatusPair> = pairs.into_iter().collect();
        Self {
            nl: Status::worst(pairs.iter().map(|pair| pair.nl)),
            fr: Status::worst(pairs.iter().map(|pair| pair.fr)),
        }
    }

    pub fn needs_attention(&self) -> bool {
        [self.nl, self.fr].iter().any(|status| matches!(status, Status::Missing | Status::Stale | Status::Parked))
    }
}

/// Why a save or sweep sent nothing to DeepL (Decision 20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Notice {
    /// `DEEPL_API_KEY` isn't set.
    NotConfigured,
    /// The owner switched automatic translation off.
    Disabled,
    /// Past the stop line on DeepL's allowance.
    Paused,
    /// More than `SAVE_CHAR_CAP` characters in one save.
    TooLong,
    /// DeepL's usage couldn't be read, so nothing is sent (fail closed).
    UsageUnknown,
    /// DeepL answered with an error or timed out.
    Failed,
}

/// `#[serde(default)]` for a `translations` field: an empty object.
pub fn empty_translations() -> Value {
    json!({})
}

/// The English fields a translation is made from: non-blank fields only.
pub fn english(fields: &[(&str, Option<&str>)]) -> Value {
    let mut map = Map::new();
    for (key, value) in fields {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            map.insert((*key).to_string(), Value::String(value.to_string()));
        }
    }
    Value::Object(map)
}

fn text_keys(english: &Value) -> Vec<String> {
    english.as_object().map(|map| map.keys().cloned().collect()).unwrap_or_default()
}

fn has_text(entry: &Value, english: &Value) -> bool {
    text_keys(english)
        .iter()
        .any(|key| entry.get(key).and_then(Value::as_str).is_some_and(|text| !text.trim().is_empty()))
}

pub fn status(translations: &Value, locale: Locale, english: &Value, keep_version: i32) -> Status {
    let Some(entry) = translations.get(locale.as_str()).filter(|entry| entry.is_object()) else {
        return Status::Missing;
    };
    let current = entry.get("source") == Some(english);
    if entry.get("manual") == Some(&Value::Bool(true)) {
        return if current { Status::Manual } else { Status::ManualStale };
    }
    if let Some(pending) = entry.get("pending")
        && pending.get("source") == Some(english)
        && pending.get("attempts").and_then(Value::as_u64).unwrap_or(0) >= MAX_ATTEMPTS
    {
        return Status::Parked;
    }
    let same_keep = entry.get("keep").and_then(Value::as_i64) == Some(i64::from(keep_version));
    match (current && same_keep, has_text(entry, english)) {
        (true, true) => Status::Ok,
        (_, false) => Status::Missing,
        (false, true) => Status::Stale,
    }
}

/// Accepts the admin form's `translations`: a locale marked `edited` becomes
/// the owner's own (`manual`, made from the current English, never
/// overwritten); a manual locale emptied by the owner goes back to
/// automatic. Unknown locales and keys are dropped.
pub fn accept_owner_edits(submitted: &Value, english: &Value) -> Value {
    let mut out = Map::new();
    for locale in TARGETS {
        let Some(entry) = submitted.get(locale.as_str()).and_then(Value::as_object) else { continue };
        let edited = entry.get("edited") == Some(&Value::Bool(true));
        let manual = entry.get("manual") == Some(&Value::Bool(true));
        if edited || manual {
            let mut owner = Map::new();
            for key in text_keys(english) {
                if let Some(text) = entry.get(&key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty()) {
                    owner.insert(key, Value::String(text.to_string()));
                }
            }
            if owner.is_empty() {
                // Cleared by the owner: DeepL fills it again.
                continue;
            }
            owner.insert("manual".into(), Value::Bool(true));
            let source = if edited { english.clone() } else { entry.get("source").cloned().unwrap_or(Value::Null) };
            owner.insert("source".into(), source);
            out.insert(locale.as_str().into(), Value::Object(owner));
        } else {
            let mut kept = entry.clone();
            kept.remove("edited");
            out.insert(locale.as_str().into(), Value::Object(kept));
        }
    }
    Value::Object(out)
}

/// Re-translate (Decision 5): drops the owner's mark and the parked count
/// so the next fill translates this locale again.
pub fn clear_for_retranslate(translations: &mut Value, locale: Locale) {
    if let Some(entry) = translations.get_mut(locale.as_str()).and_then(Value::as_object_mut) {
        entry.remove("manual");
        entry.remove("pending");
        entry.remove("source");
    }
}

/// The text for `locale`, falling back to English when missing or blank.
pub fn resolve(translations: &Value, locale: Locale, key: &str, english: &str) -> String {
    if locale == Locale::En {
        return english.to_string();
    }
    translations
        .get(locale.as_str())
        .and_then(|entry| entry.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(english)
        .to_string()
}

/// As `resolve`, for optional English text (descriptions): none in English
/// means none in any language.
pub fn resolve_opt(translations: &Value, locale: Locale, key: &str, english: Option<&str>) -> Option<String> {
    english.map(|english| resolve(translations, locale, key, english))
}

/// An order line's name in the customer's language (Decision 9): `None`
/// when that's English anyway, so `item_name` alone says it.
pub fn local_snapshot(translations: &Value, locale: Locale, key: &str, english: &str) -> Option<String> {
    let local = resolve(translations, locale, key, english);
    (local != english).then_some(local)
}

/// One text in every customer language, fallback applied (Decision 7).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LocalizedText {
    pub en: String,
    pub nl: String,
    pub fr: String,
}

impl LocalizedText {
    pub fn of(translations: &Value, key: &str, english: &str) -> Self {
        Self {
            en: english.to_string(),
            nl: resolve(translations, Locale::Nl, key, english),
            fr: resolve(translations, Locale::Fr, key, english),
        }
    }
}

/// `?locale=` on the public menu routes: unknown or missing is English.
pub fn query_locale(value: Option<&str>) -> Locale {
    value.and_then(Locale::parse).unwrap_or(Locale::En)
}

// --- Keep list (Decision 6) ---

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn xml_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Whether `word` matches `text` at the start, ignoring case; returns the
/// matched length in bytes.
fn match_at(text: &str, word: &str) -> Option<usize> {
    let mut text_chars = text.char_indices();
    for word_char in word.chars() {
        let (_, text_char) = text_chars.next()?;
        if !text_char.to_lowercase().eq(word_char.to_lowercase()) {
            return None;
        }
    }
    Some(text_chars.next().map(|(index, _)| index).unwrap_or(text.len()))
}

/// Wraps each whole-word, case-insensitive match of a keep word in
/// `<keep>…</keep>` and XML-escapes the rest, for DeepL's `tag_handling=xml`
/// with `ignore_tags=keep`. Longer words win ("Puff Puff" over "Puff").
pub fn protect(text: &str, keep: &[String]) -> String {
    let mut words: Vec<&str> = keep.iter().map(|w| w.trim()).filter(|w| !w.is_empty()).collect();
    words.sort_by_key(|word| std::cmp::Reverse(word.chars().count()));

    let mut out = String::with_capacity(text.len() + 16);
    let mut index = 0;
    let mut previous: Option<char> = None;
    while index < text.len() {
        let rest = &text[index..];
        let at_word_start = !previous.is_some_and(char::is_alphanumeric);
        let matched = at_word_start
            .then(|| {
                words.iter().find_map(|word| {
                    let len = match_at(rest, word)?;
                    let next = rest[len..].chars().next();
                    (!next.is_some_and(char::is_alphanumeric)).then_some(len)
                })
            })
            .flatten();
        match matched {
            Some(len) => {
                out.push_str("<keep>");
                out.push_str(&xml_escape(&rest[..len]));
                out.push_str("</keep>");
                previous = rest[..len].chars().last();
                index += len;
            }
            None => {
                let c = rest.chars().next().expect("index is inside text");
                out.push_str(&xml_escape(&c.to_string()));
                previous = Some(c);
                index += c.len_utf8();
            }
        }
    }
    out
}

/// Undoes `protect` on DeepL's answer.
pub fn unprotect(text: &str) -> String {
    xml_unescape(&text.replace("<keep>", "").replace("</keep>", ""))
}

// --- Settings ---

#[derive(Debug, Clone, Serialize)]
pub struct TranslationSettings {
    pub keep_words: Vec<String>,
    pub keep_version: i32,
    pub auto_translate: bool,
    pub stop_at_percent: i32,
    pub updated_at: DateTime<Utc>,
}

pub async fn load_settings(executor: impl sqlx::PgExecutor<'_>) -> Result<TranslationSettings, sqlx::Error> {
    sqlx::query_as!(
        TranslationSettings,
        r#"
        SELECT keep_words, keep_version, auto_translate, stop_at_percent,
               updated_at as "updated_at: DateTime<Utc>"
        FROM translation_settings
        "#
    )
    .fetch_one(executor)
    .await
}

// --- Units: one row's English and its translations ---

/// One row's English (`english`) and its `translations`, filled in place.
#[derive(Debug, Clone)]
pub struct Unit {
    pub english: Value,
    pub translations: Value,
}

impl Unit {
    pub fn new(english: Value, translations: Value) -> Self {
        let translations = if translations.is_object() { translations } else { json!({}) };
        Self { english, translations }
    }

    pub fn status(&self, keep_version: i32) -> StatusPair {
        StatusPair::of(&self.translations, &self.english, keep_version)
    }

    fn texts_for(&self, locale: Locale, keep_version: i32) -> Vec<(String, String)> {
        if !status(&self.translations, locale, &self.english, keep_version).needs_translation() {
            return Vec::new();
        }
        self.english
            .as_object()
            .map(|map| {
                map.iter()
                    .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The characters a full fill would send for this unit (both locales).
    pub fn pending_chars(&self, keep_version: i32) -> u64 {
        TARGETS
            .iter()
            .flat_map(|locale| self.texts_for(*locale, keep_version))
            .map(|(_, text)| text.chars().count() as u64)
            .sum()
    }

    fn entry_mut(&mut self, locale: Locale) -> &mut Map<String, Value> {
        let map = self.translations.as_object_mut().expect("Unit::new keeps translations an object");
        let entry = map.entry(locale.as_str()).or_insert_with(|| json!({}));
        if !entry.is_object() {
            *entry = json!({});
        }
        entry.as_object_mut().expect("just made an object")
    }

    fn succeed(&mut self, locale: Locale, texts: Vec<(String, String)>, keep_version: i32) {
        let english = self.english.clone();
        let entry = self.entry_mut(locale);
        entry.clear();
        for (key, text) in texts {
            entry.insert(key, Value::String(text));
        }
        entry.insert("source".into(), english);
        entry.insert("keep".into(), json!(keep_version));
    }

    fn fail(&mut self, locale: Locale) {
        let english = self.english.clone();
        let entry = self.entry_mut(locale);
        let attempts = entry
            .get("pending")
            .filter(|pending| pending.get("source") == Some(&english))
            .and_then(|pending| pending.get("attempts"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        entry.insert("pending".into(), json!({ "source": english, "attempts": attempts + 1 }));
    }
}

// --- DeepL ---

#[derive(Debug)]
enum DeeplError {
    /// 456: the allowance is used up.
    Quota,
    Other,
}

struct HttpBackend {
    client: reqwest::Client,
    key: String,
    base_url: String,
}

/// For tests: answers `[nl] text`, with configurable usage and failure.
#[derive(Debug, Default)]
pub struct FakeBackend {
    pub count: AtomicU64,
    pub limit: u64,
    pub fail: bool,
    pub usage_down: bool,
    pub calls: AtomicUsize,
}

impl FakeBackend {
    pub fn new(count: u64, limit: u64) -> Self {
        Self { count: AtomicU64::new(count), limit, ..Default::default() }
    }
}

enum Backend {
    Off,
    Http(Arc<HttpBackend>),
    Fake(Arc<FakeBackend>),
}

impl Clone for Backend {
    fn clone(&self) -> Self {
        match self {
            Self::Off => Self::Off,
            Self::Http(http) => Self::Http(http.clone()),
            Self::Fake(fake) => Self::Fake(fake.clone()),
        }
    }
}

/// The host follows the key: DeepL API Free keys end in `:fx`.
pub fn deepl_base_url(key: &str) -> &'static str {
    if key.trim_end().ends_with(":fx") { "https://api-free.deepl.com" } else { "https://api.deepl.com" }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Usage {
    pub count: u64,
    pub limit: u64,
}

#[derive(Clone)]
pub struct TranslationConfig {
    backend: Backend,
    usage: Arc<Mutex<Option<(Usage, Instant)>>>,
}

impl TranslationConfig {
    /// `DEEPL_API_KEY` unset means no automatic translation: the owner types
    /// nl/fr, and English is the fallback.
    pub fn from_env() -> Self {
        let key = std::env::var("DEEPL_API_KEY").ok().filter(|v| !v.trim().is_empty());
        let backend = match key {
            Some(key) => Backend::Http(Arc::new(HttpBackend {
                client: reqwest::Client::builder().timeout(DEEPL_TIMEOUT).build().expect("reqwest client builds"),
                base_url: deepl_base_url(&key).to_string(),
                key: key.trim().to_string(),
            })),
            None => {
                tracing::info!("DEEPL_API_KEY not set: menu translations are entered by hand");
                Backend::Off
            }
        };
        Self { backend, usage: Arc::new(Mutex::new(None)) }
    }

    pub fn disabled() -> Self {
        Self { backend: Backend::Off, usage: Arc::new(Mutex::new(None)) }
    }

    pub fn fake(fake: Arc<FakeBackend>) -> Self {
        Self { backend: Backend::Fake(fake), usage: Arc::new(Mutex::new(None)) }
    }

    pub fn is_configured(&self) -> bool {
        !matches!(self.backend, Backend::Off)
    }

    async fn fetch_usage(&self) -> Result<Usage, DeeplError> {
        match &self.backend {
            Backend::Off => Err(DeeplError::Other),
            Backend::Fake(fake) => {
                if fake.usage_down {
                    return Err(DeeplError::Other);
                }
                Ok(Usage { count: fake.count.load(Ordering::SeqCst), limit: fake.limit })
            }
            Backend::Http(http) => {
                let response = http
                    .client
                    .get(format!("{}/v2/usage", http.base_url))
                    .header("Authorization", format!("DeepL-Auth-Key {}", http.key))
                    .send()
                    .await
                    .map_err(|_| DeeplError::Other)?;
                if !response.status().is_success() {
                    tracing::warn!(status = response.status().as_u16(), "deepl usage failed");
                    return Err(DeeplError::Other);
                }
                let bytes = response.bytes().await.map_err(|_| DeeplError::Other)?;
                let body: Value = serde_json::from_slice(&bytes).map_err(|_| DeeplError::Other)?;
                let count = body.get("character_count").and_then(Value::as_u64);
                let limit = body.get("character_limit").and_then(Value::as_u64);
                count.zip(limit).map(|(count, limit)| Usage { count, limit }).ok_or(DeeplError::Other)
            }
        }
    }

    /// DeepL's usage, from the cache when it's under ten minutes old.
    pub async fn usage(&self) -> Option<Usage> {
        let mut cache = self.usage.lock().await;
        self.refreshed(&mut cache).await
    }

    async fn refreshed(&self, cache: &mut Option<(Usage, Instant)>) -> Option<Usage> {
        if let Some((usage, at)) = cache
            && at.elapsed() < USAGE_TTL
        {
            return Some(*usage);
        }
        match self.fetch_usage().await {
            Ok(usage) => {
                *cache = Some((usage, Instant::now()));
                Some(usage)
            }
            Err(_) => None,
        }
    }

    /// Reserves `chars` against the allowance, or says why not (Decision 20).
    /// Reserved characters count locally until the next refresh, so a burst
    /// of saves can't overshoot between usage reads.
    async fn reserve(&self, settings: &TranslationSettings, chars: u64) -> Result<(), Notice> {
        if !self.is_configured() {
            return Err(Notice::NotConfigured);
        }
        if !settings.auto_translate {
            return Err(Notice::Disabled);
        }
        let mut cache = self.usage.lock().await;
        let Some(usage) = self.refreshed(&mut cache).await else {
            return Err(Notice::UsageUnknown);
        };
        if usage.count + chars > stop_line(usage.limit, settings.stop_at_percent) {
            return Err(Notice::Paused);
        }
        if let Some((cached, _)) = cache.as_mut() {
            cached.count += chars;
        }
        Ok(())
    }

    async fn mark_exhausted(&self) {
        if let Some((usage, _)) = self.usage.lock().await.as_mut() {
            usage.count = usage.limit;
        }
    }

    async fn translate(&self, texts: &[String], target: Locale, keep: &[String]) -> Result<Vec<String>, DeeplError> {
        let protected: Vec<String> = texts.iter().map(|text| protect(text, keep)).collect();
        let answers = match &self.backend {
            Backend::Off => return Err(DeeplError::Other),
            Backend::Fake(fake) => {
                fake.calls.fetch_add(1, Ordering::SeqCst);
                if fake.fail {
                    return Err(DeeplError::Other);
                }
                let sent: u64 = texts.iter().map(|text| text.chars().count() as u64).sum();
                fake.count.fetch_add(sent, Ordering::SeqCst);
                protected.iter().map(|text| format!("[{}] {text}", target.as_str())).collect()
            }
            Backend::Http(http) => {
                let mut body = json!({
                    "text": protected,
                    "source_lang": "EN",
                    "target_lang": target.as_str().to_uppercase(),
                    "tag_handling": "xml",
                    "ignore_tags": ["keep"],
                    "preserve_formatting": true,
                });
                if target == Locale::Fr {
                    body["formality"] = json!("prefer_more");
                }
                let response = http
                    .client
                    .post(format!("{}/v2/translate", http.base_url))
                    .header("Authorization", format!("DeepL-Auth-Key {}", http.key))
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                    .send()
                    .await
                    .map_err(|error| {
                        tracing::warn!(timeout = error.is_timeout(), "deepl translate failed");
                        DeeplError::Other
                    })?;
                let status = response.status().as_u16();
                if status == 456 {
                    tracing::warn!(status, "deepl allowance used up");
                    return Err(DeeplError::Quota);
                }
                if !response.status().is_success() {
                    tracing::warn!(status, "deepl translate failed");
                    return Err(DeeplError::Other);
                }
                let bytes = response.bytes().await.map_err(|_| DeeplError::Other)?;
                let body: Value = serde_json::from_slice(&bytes).map_err(|_| DeeplError::Other)?;
                let answers: Vec<String> = body
                    .get("translations")
                    .and_then(Value::as_array)
                    .map(|list| list.iter().filter_map(|t| t.get("text")?.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                if answers.len() != texts.len() {
                    tracing::warn!(sent = texts.len(), got = answers.len(), "deepl answer count mismatch");
                    return Err(DeeplError::Other);
                }
                answers
            }
        };
        Ok(answers.iter().map(|text| unprotect(text)).collect())
    }

    /// Translates every locale of `units` that needs it: one DeepL request
    /// per locale for all units together. Returns the characters sent and,
    /// if anything was held back or failed, why. `cap` bounds one request.
    pub async fn fill(&self, settings: &TranslationSettings, units: &mut [Unit], cap: u64) -> (u64, Option<Notice>) {
        let mut sent = 0;
        let mut notice = None;
        for locale in TARGETS {
            let mut wanted: Vec<(usize, String, String)> = Vec::new();
            for (index, unit) in units.iter().enumerate() {
                for (key, text) in unit.texts_for(locale, settings.keep_version) {
                    wanted.push((index, key, text));
                }
            }
            if wanted.is_empty() {
                continue;
            }
            let chars: u64 = wanted.iter().map(|(_, _, text)| text.chars().count() as u64).sum();
            if chars > cap {
                notice = Some(Notice::TooLong);
                continue;
            }
            if let Err(refusal) = self.reserve(settings, chars).await {
                return (sent, Some(refusal));
            }
            let texts: Vec<String> = wanted.iter().map(|(_, _, text)| text.clone()).collect();
            match self.translate(&texts, locale, &settings.keep_words).await {
                Ok(answers) => {
                    sent += chars;
                    let mut by_unit: Vec<Vec<(String, String)>> = vec![Vec::new(); units.len()];
                    for ((index, key, _), answer) in wanted.into_iter().zip(answers) {
                        by_unit[index].push((key, answer));
                    }
                    for (index, texts) in by_unit.into_iter().enumerate() {
                        if !texts.is_empty() {
                            units[index].succeed(locale, texts, settings.keep_version);
                        }
                    }
                }
                Err(error) => {
                    let mut failed: Vec<usize> = wanted.iter().map(|(index, _, _)| *index).collect();
                    failed.dedup();
                    for index in failed {
                        units[index].fail(locale);
                    }
                    if matches!(error, DeeplError::Quota) {
                        self.mark_exhausted().await;
                        return (sent, Some(Notice::Paused));
                    }
                    notice = Some(Notice::Failed);
                }
            }
        }
        (sent, notice)
    }
}

/// The most characters DeepL may have counted before we stop sending.
pub fn stop_line(limit: u64, stop_at_percent: i32) -> u64 {
    limit.saturating_mul(stop_at_percent.clamp(0, 100) as u64) / 100
}

/// Every save: keep the owner's own nl/fr edits.
pub fn accept_edits(units: &mut [Unit]) {
    for unit in units.iter_mut() {
        unit.translations = accept_owner_edits(&unit.translations, &unit.english);
    }
}

/// An explicit save (rentals) or a publish: the owner's edits, then DeepL
/// for what's missing or stale. A refusal or failure never fails the save
/// (Decision 3). Autosaved drafts only `accept_edits`: translating every
/// pause in typing would spend the allowance on half-typed text.
pub async fn prepare(
    config: &TranslationConfig,
    pool: &PgPool,
    units: &mut [Unit],
) -> Result<Option<Notice>, sqlx::Error> {
    accept_edits(units);
    let settings = load_settings(pool).await?;
    let (_, notice) = config.fill(&settings, units, SAVE_CHAR_CAP).await;
    Ok(notice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(auto: bool, stop: i32) -> TranslationSettings {
        TranslationSettings {
            keep_words: vec!["Jollof".into(), "Puff Puff".into(), "Puff".into()],
            keep_version: 1,
            auto_translate: auto,
            stop_at_percent: stop,
            updated_at: Utc::now(),
        }
    }

    fn unit(name: &str) -> Unit {
        Unit::new(english(&[("name", Some(name)), ("description", None)]), json!({}))
    }

    #[test]
    fn protect_wraps_whole_words_and_escapes() {
        let keep = vec!["Jollof".to_string(), "Puff Puff".to_string(), "Puff".to_string()];
        assert_eq!(protect("Jollof Rice", &keep), "<keep>Jollof</keep> Rice");
        assert_eq!(protect("jollof & chicken", &keep), "<keep>jollof</keep> &amp; chicken");
        assert_eq!(protect("Puff Puff (small)", &keep), "<keep>Puff Puff</keep> (small)");
        assert_eq!(protect("Jolloffy <b>", &keep), "Jolloffy &lt;b&gt;");
        assert_eq!(protect("Pépé Jollof", &keep), "Pépé <keep>Jollof</keep>");
    }

    #[test]
    fn unprotect_round_trips() {
        let keep = vec!["Jollof".to_string()];
        for text in ["Jollof Rice & <stew>", "Rijst met jollof", "Gâteau \"maison\""] {
            assert_eq!(unprotect(&protect(text, &keep)), text);
        }
        assert_eq!(unprotect("<keep>Jollof</keep> rijst &amp; kip"), "Jollof rijst & kip");
    }

    #[test]
    fn resolve_falls_back_to_english() {
        let translations = json!({ "nl": { "name": "Rijst", "description": "  " } });
        assert_eq!(resolve(&translations, Locale::Nl, "name", "Rice"), "Rijst");
        assert_eq!(resolve(&translations, Locale::Nl, "description", "Tasty"), "Tasty");
        assert_eq!(resolve(&translations, Locale::Fr, "name", "Rice"), "Rice");
        assert_eq!(resolve(&translations, Locale::En, "name", "Rice"), "Rice");
        assert_eq!(resolve_opt(&translations, Locale::Nl, "description", None), None);
    }

    #[test]
    fn host_follows_the_key() {
        assert_eq!(deepl_base_url("abc:fx"), "https://api-free.deepl.com");
        assert_eq!(deepl_base_url("abc"), "https://api.deepl.com");
    }

    #[test]
    fn status_tracks_source_keep_and_manual() {
        let en = english(&[("name", Some("Rice"))]);
        let ok = json!({ "nl": { "name": "Rijst", "source": en, "keep": 1 } });
        assert_eq!(status(&ok, Locale::Nl, &en, 1), Status::Ok);
        assert_eq!(status(&ok, Locale::Nl, &en, 2), Status::Stale);
        assert_eq!(status(&ok, Locale::Fr, &en, 1), Status::Missing);
        let changed = english(&[("name", Some("Fried rice"))]);
        assert_eq!(status(&ok, Locale::Nl, &changed, 1), Status::Stale);
        let manual = json!({ "nl": { "name": "Rijst", "source": en, "manual": true } });
        assert_eq!(status(&manual, Locale::Nl, &en, 2), Status::Manual);
        assert_eq!(status(&manual, Locale::Nl, &changed, 1), Status::ManualStale);
    }

    #[test]
    fn owner_edits_become_manual_and_clearing_returns_to_automatic() {
        let en = english(&[("name", Some("Rice")), ("description", Some("Plain"))]);
        let submitted = json!({
            "nl": { "name": " Rijst ", "edited": true },
            "fr": { "name": "", "description": "", "manual": true },
            "de": { "name": "Reis", "edited": true }
        });
        let accepted = accept_owner_edits(&submitted, &en);
        assert_eq!(accepted["nl"], json!({ "name": "Rijst", "manual": true, "source": en }));
        assert!(accepted.get("fr").is_none());
        assert!(accepted.get("de").is_none());
    }

    #[test]
    fn stop_line_is_a_percentage_of_the_limit() {
        assert_eq!(stop_line(1_000_000, 80), 800_000);
        assert_eq!(stop_line(1_000_000, 100), 1_000_000);
    }

    #[tokio::test]
    async fn fill_translates_both_locales_and_keeps_words() {
        let fake = Arc::new(FakeBackend::new(0, 1_000_000));
        let config = TranslationConfig::fake(fake.clone());
        let mut units = vec![unit("Jollof Rice")];
        let (sent, notice) = config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await;
        assert_eq!(notice, None);
        assert_eq!(sent, 22);
        assert_eq!(units[0].translations["nl"]["name"], "[nl] Jollof Rice");
        assert_eq!(units[0].translations["fr"]["name"], "[fr] Jollof Rice");
        assert_eq!(units[0].status(1).nl, Status::Ok);
        // Nothing left to do: a second fill sends nothing.
        let (sent, _) = config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await;
        assert_eq!(sent, 0);
        assert_eq!(fake.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn fill_stops_at_the_stop_line() {
        let fake = Arc::new(FakeBackend::new(799_990, 1_000_000));
        let config = TranslationConfig::fake(fake.clone());
        let mut units = vec![unit("Fried plantain")];
        let (sent, notice) = config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await;
        assert_eq!((sent, notice), (0, Some(Notice::Paused)));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn reserved_characters_count_before_the_next_usage_read() {
        let fake = Arc::new(FakeBackend::new(799_970, 1_000_000));
        let config = TranslationConfig::fake(fake.clone());
        // "Rice" × 2 locales = 8 characters fit; the cached count then blocks
        // the second item even though the fake's count isn't re-read.
        let mut first = vec![unit("Rice")];
        assert_eq!(config.fill(&settings(true, 80), &mut first, SAVE_CHAR_CAP).await.1, None);
        let mut second = vec![unit("A much longer dish name")];
        assert_eq!(config.fill(&settings(true, 80), &mut second, SAVE_CHAR_CAP).await.1, Some(Notice::Paused));
    }

    #[tokio::test]
    async fn fill_refuses_when_off_unknown_or_too_long() {
        let fake = Arc::new(FakeBackend::new(0, 1_000_000));
        let config = TranslationConfig::fake(fake.clone());
        let mut units = vec![unit("Rice")];
        assert_eq!(config.fill(&settings(false, 80), &mut units, SAVE_CHAR_CAP).await.1, Some(Notice::Disabled));
        assert_eq!(config.fill(&settings(true, 80), &mut units, 3).await.1, Some(Notice::TooLong));

        let down = Arc::new(FakeBackend { usage_down: true, limit: 1_000_000, ..Default::default() });
        let config = TranslationConfig::fake(down.clone());
        assert_eq!(config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await.1, Some(Notice::UsageUnknown));
        assert_eq!(down.calls.load(Ordering::SeqCst), 0);

        let off = TranslationConfig::disabled();
        assert_eq!(off.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await.1, Some(Notice::NotConfigured));
    }

    #[tokio::test]
    async fn a_unit_is_parked_after_three_failures() {
        let fake = Arc::new(FakeBackend { fail: true, limit: 1_000_000, ..Default::default() });
        let config = TranslationConfig::fake(fake.clone());
        let mut units = vec![unit("Rice")];
        for _ in 0..MAX_ATTEMPTS {
            assert_eq!(config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await.1, Some(Notice::Failed));
        }
        assert_eq!(units[0].status(1).nl, Status::Parked);
        let calls = fake.calls.load(Ordering::SeqCst);
        config.fill(&settings(true, 80), &mut units, SAVE_CHAR_CAP).await;
        assert_eq!(fake.calls.load(Ordering::SeqCst), calls, "parked units aren't retried");

        clear_for_retranslate(&mut units[0].translations, Locale::Nl);
        assert_eq!(units[0].status(1).nl, Status::Missing);
    }
}
