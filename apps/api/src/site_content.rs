//! The admin-editable site content document (Phase 25 Decisions 11–13):
//! replaceable site images and per-locale homepage text overrides. A
//! missing or null value falls back to the web's bundled default, so an
//! empty document renders the site exactly as before.

use crate::error::FieldError;
use crate::order_emails::Locale;
use crate::routes::admin::menu_items::is_valid_media_url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SiteContent {
    #[serde(default)]
    pub images: SiteImages,
    /// locale → `HomePage` message key → text.
    #[serde(default)]
    pub home_text: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SiteImages {
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub hero: Option<String>,
    #[serde(default)]
    pub category_meals: Option<String>,
    #[serde(default)]
    pub category_snacks: Option<String>,
    #[serde(default)]
    pub category_catering: Option<String>,
    #[serde(default)]
    pub category_event_rentals: Option<String>,
    #[serde(default)]
    pub about_hero: Option<String>,
    #[serde(default)]
    pub about_mission: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
}

impl SiteImages {
    /// Every slot with its name, in the order the admin lists them.
    pub fn slots(&self) -> [(&'static str, Option<&str>); 9] {
        [
            ("logo", self.logo.as_deref()),
            ("hero", self.hero.as_deref()),
            ("category_meals", self.category_meals.as_deref()),
            ("category_snacks", self.category_snacks.as_deref()),
            ("category_catering", self.category_catering.as_deref()),
            ("category_event_rentals", self.category_event_rentals.as_deref()),
            ("about_hero", self.about_hero.as_deref()),
            ("about_mission", self.about_mission.as_deref()),
            ("contact", self.contact.as_deref()),
        ]
    }

    fn slots_mut(&mut self) -> [&mut Option<String>; 9] {
        [
            &mut self.logo,
            &mut self.hero,
            &mut self.category_meals,
            &mut self.category_snacks,
            &mut self.category_catering,
            &mut self.category_event_rentals,
            &mut self.about_hero,
            &mut self.about_mission,
            &mut self.contact,
        ]
    }

    pub fn urls(&self) -> impl Iterator<Item = &str> {
        self.slots().into_iter().filter_map(|(_, url)| url)
    }
}

/// The editable `HomePage` keys: all of them except the SEO
/// `metaTitle`/`metaDescription` (Phase 26). The web's `HomeTextKey` union
/// mirrors this list, and `pnpm check:messages` keeps the two in step.
pub const HOME_TEXT_KEYS: &[&str] = &[
    "heroEyebrow", "heroHeadline", "heroSupporting", "orderNow", "bookCatering",
    "whatsappUs", "trustHalal", "trustFresh", "trustHygienic", "trustReliable",
    "trustHalalDesc", "trustFreshDesc", "trustHygienicDesc", "trustReliableDesc",
    "servicesMeals", "servicesSnacks", "servicesCatering", "servicesEventRentals",
    "whyChooseUsEyebrow", "whyChooseUs", "cateringCtaHeadline", "requestCateringQuote",
    "aboutPreview", "learnMore", "featuredMenuEyebrow", "featuredMenuHeading", "viewFullMenu",
];

/// Keys rendered with `t.rich`, which may carry `<em>…</em>`.
const RICH_KEYS: &[&str] = &["heroHeadline", "whyChooseUs"];

const MAX_TEXT_CHARS: usize = 400;

impl SiteContent {
    /// Trims every value and drops empty ones, so an empty field removes
    /// the override (Decision 12).
    pub fn normalize(mut self) -> Self {
        for slot in self.images.slots_mut() {
            *slot = slot.take().map(|url| url.trim().to_string()).filter(|url| !url.is_empty());
        }
        for texts in self.home_text.values_mut() {
            texts.retain(|_, text| {
                *text = text.trim().to_string();
                !text.is_empty()
            });
        }
        self.home_text.retain(|_, texts| !texts.is_empty());
        self
    }

    /// `previous` holds every image URL already stored (published or
    /// draft): only a new URL has to come from this API's own upload
    /// endpoint, like menu photos (Phase 14 Decision 14 addendum).
    pub fn validate(&self, previous: &[&SiteContent], public_base_url: &str) -> Vec<FieldError> {
        let mut errors = Vec::new();
        let error = |field: String, message: &str| FieldError { field, message: message.into() };

        for (locale, texts) in &self.home_text {
            if Locale::parse(locale).is_none() {
                errors.push(error(format!("home_text.{locale}"), "invalid_locale"));
                continue;
            }
            for (key, text) in texts {
                let field = format!("home_text.{locale}.{key}");
                if !HOME_TEXT_KEYS.contains(&key.as_str()) {
                    errors.push(error(field, "unknown_key"));
                } else if text.chars().count() > MAX_TEXT_CHARS {
                    errors.push(error(field, "too_long"));
                } else if !markup_allowed(key, text) {
                    errors.push(error(field, "invalid_markup"));
                }
            }
        }

        for (slot, url) in self.images.slots() {
            let Some(url) = url else { continue };
            let already_stored =
                previous.iter().any(|content| content.images.urls().any(|stored| stored == url));
            if !already_stored && !is_valid_media_url(url, public_base_url) {
                errors.push(error(format!("images.{slot}"), "invalid"));
            }
        }

        errors
    }
}

/// Plain keys allow no `<` at all. Rich keys allow only `<em>`/`</em>`,
/// balanced and not nested, which is all `t.rich`'s tag map renders.
fn markup_allowed(key: &str, text: &str) -> bool {
    if !RICH_KEYS.contains(&key) {
        return !text.contains('<');
    }

    let mut rest = text;
    let mut open = false;
    while let Some(index) = rest.find('<') {
        let tag = &rest[index..];
        if !open && tag.starts_with("<em>") {
            open = true;
            rest = &tag["<em>".len()..];
        } else if open && tag.starts_with("</em>") {
            open = false;
            rest = &tag["</em>".len()..];
        } else {
            return false;
        }
    }
    !open
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "http://localhost:8080";

    fn with_text(locale: &str, key: &str, text: &str) -> SiteContent {
        let mut content = SiteContent::default();
        content.home_text.entry(locale.into()).or_default().insert(key.into(), text.into());
        content
    }

    fn fields(content: &SiteContent) -> Vec<String> {
        content.validate(&[], BASE).into_iter().map(|e| e.field).collect()
    }

    #[test]
    fn site_content_accepts_em_in_rich_keys_only() {
        assert!(fields(&with_text("fr", "heroHeadline", "Le <em>goût</em> du Nigeria")).is_empty());
        assert_eq!(fields(&with_text("fr", "heroEyebrow", "<em>x</em>")), ["home_text.fr.heroEyebrow"]);
    }

    #[test]
    fn site_content_rejects_other_or_unbalanced_tags() {
        for text in ["<script>alert(1)</script>", "<em>open", "close</em>", "<em><em>x</em></em>", "a < b"] {
            assert_eq!(fields(&with_text("en", "heroHeadline", text)), ["home_text.en.heroHeadline"], "{text}");
        }
    }

    #[test]
    fn site_content_rejects_unknown_locales_and_keys() {
        assert_eq!(fields(&with_text("de", "heroEyebrow", "Hallo")), ["home_text.de"]);
        assert_eq!(fields(&with_text("en", "metaTitle", "SEO")), ["home_text.en.metaTitle"]);
        assert_eq!(fields(&with_text("en", "heroEyebrow", &"x".repeat(401))), ["home_text.en.heroEyebrow"]);
    }

    #[test]
    fn site_content_images_must_come_from_this_api_unless_already_stored() {
        let mut content = SiteContent::default();
        content.images.hero = Some("https://evil.example/hero.jpg".into());
        assert_eq!(fields(&content), ["images.hero"]);

        content.images.hero = Some(format!("{BASE}/api/media/abc.jpg"));
        assert!(fields(&content).is_empty());

        let mut stored = SiteContent::default();
        stored.images.logo = Some("/legacy/logo.png".into());
        content.images.logo = Some("/legacy/logo.png".into());
        assert!(content.validate(&[&stored], BASE).is_empty());
    }

    #[test]
    fn site_content_normalize_trims_and_drops_empty_values() {
        let mut content = with_text("en", "heroEyebrow", "  Hello  ");
        content.home_text.entry("nl".into()).or_default().insert("heroEyebrow".into(), "   ".into());
        content.images.hero = Some(" ".into());
        let content = content.normalize();
        assert_eq!(content.home_text["en"]["heroEyebrow"], "Hello");
        assert!(!content.home_text.contains_key("nl"));
        assert_eq!(content.images.hero, None);
    }

    #[test]
    fn site_content_rejects_unknown_fields() {
        assert!(serde_json::from_str::<SiteContent>(r#"{"images":{"banner":null}}"#).is_err());
        assert!(serde_json::from_str::<SiteContent>(r#"{"extra":1}"#).is_err());
        assert_eq!(serde_json::from_str::<SiteContent>("{}").unwrap(), SiteContent::default());
    }
}
