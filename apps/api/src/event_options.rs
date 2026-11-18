//! Plate option groups (Phase 32 Decisions 1-4, 7): "Choose your protein:
//! assorted meat or fish". Each group has an admin-set min and max picks
//! (Phase 35 Decisions 7-8, default exactly one), and choices never change
//! the price. Groups live as JSON on the plate; each guest's picks are
//! snapshotted with their English label.

use crate::order_emails::Locale;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// `{"en": "Protein", "nl": "Eiwit", "fr": "Protéine"}`. English is
/// required; nl and fr fall back to it.
pub type Labels = BTreeMap<String, String>;

pub fn label_in(labels: &Labels, locale: Locale) -> String {
    let pick = |code: &str| labels.get(code).map(|text| text.trim()).filter(|text| !text.is_empty());
    pick(locale.as_str()).or_else(|| pick("en")).unwrap_or_default().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionChoice {
    #[serde(default)]
    pub key: String,
    pub label: Labels,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionGroup {
    #[serde(default)]
    pub key: String,
    pub label: Labels,
    pub choices: Vec<OptionChoice>,
    /// Fewest and most picks per guest; Phase 32 groups have neither and
    /// read as exactly one.
    #[serde(default = "one")]
    pub min: u8,
    #[serde(default = "one")]
    pub max: u8,
}

fn one() -> u8 {
    1
}

/// A guest's picks in one group. A bare string is the Phase 32 shape, still
/// accepted from a cached page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Picked {
    One(String),
    Many(Vec<String>),
}

impl Picked {
    pub fn keys(&self) -> &[String] {
        match self {
            Picked::One(key) => std::slice::from_ref(key),
            Picked::Many(keys) => keys,
        }
    }
}

/// One guest's pick in one group, with the choice's English label at
/// booking time (Decision 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestChoice {
    pub group: String,
    pub choice: String,
    pub label: String,
}

/// Reads a plate's `options` column, treating anything malformed as no
/// options rather than failing the page.
pub fn parse_groups(value: &serde_json::Value) -> Vec<OptionGroup> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

pub fn parse_choices(value: &serde_json::Value) -> Vec<GuestChoice> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// Checks a guest's picks against the plate's groups: known, distinct
/// choices, between the group's min and max, nothing else. Picks come back
/// in the plate's order, not the guest's, so equal picks snapshot equal.
/// The error is the field message.
pub fn resolve_choices(
    groups: &[OptionGroup],
    picked: &BTreeMap<String, Picked>,
) -> Result<Vec<GuestChoice>, &'static str> {
    if picked.keys().any(|key| !groups.iter().any(|group| &group.key == key)) {
        return Err("unknown_option");
    }
    let mut resolved = Vec::new();
    for group in groups {
        let keys = picked.get(&group.key).map(Picked::keys).unwrap_or_default();
        let mut seen = HashSet::new();
        if keys.iter().any(|key| !seen.insert(key) || !group.choices.iter().any(|choice| &choice.key == key)) {
            return Err("unknown_option");
        }
        if keys.len() < usize::from(group.min) {
            return Err("option_required");
        }
        if keys.len() > usize::from(group.max) {
            return Err("too_many_options");
        }
        resolved.extend(group.choices.iter().filter(|choice| keys.contains(&choice.key)).map(|choice| GuestChoice {
            group: group.key.clone(),
            choice: choice.key.clone(),
            label: label_in(&choice.label, Locale::En),
        }));
    }
    Ok(resolved)
}

/// "Fried plantain, Chicken": the guest's picks in `locale`, translated
/// from the plate's groups while the choice still exists, else the English
/// snapshot.
pub fn choices_summary(choices: &[GuestChoice], groups: &[OptionGroup], locale: Locale) -> String {
    choices
        .iter()
        .map(|picked| {
            groups
                .iter()
                .find(|group| group.key == picked.group)
                .and_then(|group| group.choices.iter().find(|choice| choice.key == picked.choice))
                .map(|choice| label_in(&choice.label, locale))
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| picked.label.clone())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

const MAX_GROUPS: usize = 4;
const MAX_CHOICES: usize = 12;

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 40
        && key.split('-').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn clean_labels(labels: &Labels) -> Labels {
    Locale::ALL
        .iter()
        .map(|locale| {
            let text = labels.get(locale.as_str()).map(|text| text.trim()).unwrap_or_default();
            (locale.as_str().to_string(), text.to_string())
        })
        .collect()
}

/// `base`, or `base-2`, `base-3`… if it's taken.
fn unique_key(base: &str, taken: &HashSet<String>) -> String {
    let base = if base.len() > 30 { base[..30].trim_end_matches('-').to_string() } else { base.to_string() };
    if !taken.contains(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|key| !taken.contains(key)).expect("a free key")
}

/// Trims labels to the three locales and gives new groups and choices a
/// key from their English label. Existing keys are kept as sent, so
/// snapshots and counts stay stable (Decision 7).
pub fn normalize_groups(groups: &[OptionGroup]) -> Vec<OptionGroup> {
    let mut group_keys = HashSet::new();
    groups
        .iter()
        .map(|group| {
            let label = clean_labels(&group.label);
            let key = match group.key.trim() {
                "" => unique_key(&crate::events::slugify(&label_in(&label, Locale::En)), &group_keys),
                key => key.to_string(),
            };
            group_keys.insert(key.clone());
            let mut choice_keys = HashSet::new();
            let choices = group
                .choices
                .iter()
                .map(|choice| {
                    let label = clean_labels(&choice.label);
                    let key = match choice.key.trim() {
                        "" => unique_key(&crate::events::slugify(&label_in(&label, Locale::En)), &choice_keys),
                        key => key.to_string(),
                    };
                    choice_keys.insert(key.clone());
                    OptionChoice { key, label }
                })
                .collect();
            OptionGroup { key, label, choices, min: group.min, max: group.max }
        })
        .collect()
}

/// Field errors (`(suffix, message)`) for a plate's normalized groups:
/// English labels, valid unique keys, two or more choices per group, and
/// `0 <= min <= max <= choices` with at least one pick allowed.
pub fn validate_groups(groups: &[OptionGroup]) -> Vec<(String, &'static str)> {
    let mut errors = Vec::new();
    if groups.len() > MAX_GROUPS {
        errors.push(("options".to_string(), "too_many"));
    }
    let mut group_keys = HashSet::new();
    for (g, group) in groups.iter().enumerate() {
        if label_in(&group.label, Locale::En).is_empty() {
            errors.push((format!("options[{g}].label"), "required"));
        }
        if !is_valid_key(&group.key) || !group_keys.insert(group.key.as_str()) {
            errors.push((format!("options[{g}].key"), "invalid"));
        }
        if group.choices.len() < 2 {
            errors.push((format!("options[{g}].choices"), "too_few"));
        } else if group.choices.len() > MAX_CHOICES {
            errors.push((format!("options[{g}].choices"), "too_many"));
        }
        if group.min > group.max || group.max == 0 || usize::from(group.max) > group.choices.len() {
            errors.push((format!("options[{g}].limits"), "invalid"));
        }
        let mut choice_keys = HashSet::new();
        for (c, choice) in group.choices.iter().enumerate() {
            if label_in(&choice.label, Locale::En).is_empty() {
                errors.push((format!("options[{g}].choices[{c}].label"), "required"));
            }
            if !is_valid_key(&choice.key) || !choice_keys.insert(choice.key.as_str()) {
                errors.push((format!("options[{g}].choices[{c}].key"), "invalid"));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn groups() -> Vec<OptionGroup> {
        parse_groups(&json!([
            {"key": "side", "label": {"en": "Side", "nl": "Bijgerecht"}, "choices": [
                {"key": "plantain", "label": {"en": "Fried plantain", "nl": "Gebakken bakbanaan"}},
                {"key": "salad", "label": {"en": "Fresh salad"}}]},
            {"key": "protein", "label": {"en": "Protein"}, "choices": [
                {"key": "chicken", "label": {"en": "Chicken", "nl": "Kip"}},
                {"key": "turkey", "label": {"en": "Turkey", "nl": "Kalkoen"}}]}
        ]))
    }

    fn picks(pairs: &[(&str, &str)]) -> BTreeMap<String, Picked> {
        pairs.iter().map(|(k, v)| (k.to_string(), Picked::One(v.to_string()))).collect()
    }

    fn many(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Picked> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Picked::Many(v.iter().map(|key| key.to_string()).collect())))
            .collect()
    }

    /// Five sides, pick one to three.
    fn sides(min: u8, max: u8) -> Vec<OptionGroup> {
        parse_groups(&json!([
            {"key": "side", "label": {"en": "Side"}, "min": min, "max": max, "choices": [
                {"key": "plantain", "label": {"en": "Fried plantain"}},
                {"key": "moin-moin", "label": {"en": "Moin moin"}},
                {"key": "salad", "label": {"en": "Fresh salad"}},
                {"key": "rice", "label": {"en": "White rice"}},
                {"key": "beans", "label": {"en": "Beans"}}]}
        ]))
    }

    #[test]
    fn phase_32_groups_default_to_exactly_one() {
        assert!(groups().iter().all(|group| group.min == 1 && group.max == 1));
    }

    #[test]
    fn up_to_three_of_five() {
        let groups = sides(1, 3);
        assert_eq!(resolve_choices(&groups, &many(&[("side", &["salad"])])).unwrap().len(), 1);
        let three = resolve_choices(&groups, &many(&[("side", &["salad", "plantain", "beans"])])).unwrap();
        let keys: Vec<_> = three.iter().map(|c| c.choice.as_str()).collect();
        assert_eq!(keys, ["plantain", "salad", "beans"], "plate order, not click order");
        assert_eq!(
            resolve_choices(&groups, &many(&[("side", &["salad", "plantain", "beans", "rice"])])),
            Err("too_many_options")
        );
        assert_eq!(resolve_choices(&groups, &many(&[("side", &[])])), Err("option_required"));
        assert_eq!(resolve_choices(&groups, &many(&[])), Err("option_required"));
        assert_eq!(resolve_choices(&groups, &many(&[("side", &["salad", "salad"])])), Err("unknown_option"));
    }

    #[test]
    fn exactly_two_and_optional_groups() {
        let exactly = sides(2, 2);
        assert_eq!(resolve_choices(&exactly, &many(&[("side", &["salad"])])), Err("option_required"));
        assert!(resolve_choices(&exactly, &many(&[("side", &["salad", "rice"])])).is_ok());
        assert_eq!(
            resolve_choices(&exactly, &many(&[("side", &["salad", "rice", "beans"])])),
            Err("too_many_options")
        );
        assert_eq!(resolve_choices(&sides(0, 2), &many(&[])), Ok(vec![]));
    }

    #[test]
    fn validation_checks_limits() {
        let errors = |min, max| validate_groups(&normalize_groups(&sides(min, max)));
        assert!(errors(0, 3).is_empty());
        assert!(errors(5, 5).is_empty());
        let invalid = ("options[0].limits".to_string(), "invalid");
        assert!(errors(2, 1).contains(&invalid));
        assert!(errors(0, 0).contains(&invalid));
        assert!(errors(1, 6).contains(&invalid));
    }

    #[test]
    fn resolves_one_choice_per_group_in_group_order() {
        let resolved = resolve_choices(&groups(), &picks(&[("protein", "turkey"), ("side", "plantain")])).unwrap();
        assert_eq!(
            resolved,
            vec![
                GuestChoice { group: "side".into(), choice: "plantain".into(), label: "Fried plantain".into() },
                GuestChoice { group: "protein".into(), choice: "turkey".into(), label: "Turkey".into() },
            ]
        );
    }

    #[test]
    fn rejects_missing_unknown_and_extra_picks() {
        assert_eq!(resolve_choices(&groups(), &picks(&[("side", "salad")])), Err("option_required"));
        assert_eq!(
            resolve_choices(&groups(), &picks(&[("side", "rice"), ("protein", "chicken")])),
            Err("unknown_option")
        );
        assert_eq!(
            resolve_choices(&groups(), &picks(&[("side", "salad"), ("protein", "chicken"), ("drink", "tea")])),
            Err("unknown_option")
        );
        assert_eq!(resolve_choices(&[], &picks(&[])), Ok(vec![]));
        assert_eq!(resolve_choices(&[], &picks(&[("side", "salad")])), Err("unknown_option"));
    }

    #[test]
    fn summary_translates_and_falls_back_to_the_snapshot() {
        let choices = resolve_choices(&groups(), &picks(&[("side", "salad"), ("protein", "chicken")])).unwrap();
        assert_eq!(choices_summary(&choices, &groups(), Locale::Nl), "Fresh salad, Kip");
        assert_eq!(choices_summary(&choices, &[], Locale::Nl), "Fresh salad, Chicken");
    }

    #[test]
    fn normalize_keys_new_entries_and_keeps_existing_keys() {
        let input = parse_groups(&json!([
            {"label": {"en": " Protein "}, "choices": [
                {"key": "meat", "label": {"en": "Assorted meat"}},
                {"label": {"en": "Fish"}},
                {"label": {"en": "Fish"}}]}
        ]));
        let groups = normalize_groups(&input);
        assert_eq!(groups[0].key, "protein");
        assert_eq!(groups[0].label["en"], "Protein");
        assert_eq!(groups[0].label["fr"], "");
        let keys: Vec<_> = groups[0].choices.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["meat", "fish", "fish-2"]);
        assert!(validate_groups(&groups).is_empty());
    }

    #[test]
    fn validation_needs_labels_and_two_choices() {
        let groups = normalize_groups(&parse_groups(&json!([
            {"label": {"en": ""}, "choices": [{"label": {"en": "Only"}}]}
        ])));
        let errors = validate_groups(&groups);
        assert!(errors.contains(&("options[0].label".to_string(), "required")));
        assert!(errors.contains(&("options[0].choices".to_string(), "too_few")));
    }
}
