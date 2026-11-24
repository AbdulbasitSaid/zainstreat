//! Phase 38 Decision 4: every five minutes, translate live menu rows whose
//! nl or fr is missing or stale — the first run after deploy fills today's
//! menu, later runs pick up publishes DeepL missed. Drafts and unpublished
//! rows are left to Publish (Decision 2): an open admin form would
//! overwrite what the sweep wrote, and publish would pay for it again. Bounded by `SWEEP_CHAR_CAP` per
//! run and by the allowance guard (Decision 20).
//!
//! Writes never touch `updated_at` or `draft_updated_at` (the admin forms'
//! concurrency tokens). Instead each write only lands if the row's English
//! and translations are still what was read, so an owner edit in between
//! is never overwritten, and an unrelated change (an availability toggle)
//! doesn't waste the characters already sent.

use crate::routes::admin::categories::CategoryFields;
use crate::routes::admin::menu_items::MenuItemFields;
use crate::translation::{self, Notice, SWEEP_CHAR_CAP, TranslationConfig, TranslationSettings, Unit};
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;

const SWEEP_EVERY: Duration = Duration::from_secs(300);

/// One translatable row, and whether the sweep may fill it.
struct Entry {
    kind: &'static str,
    id: i64,
    sweepable: bool,
    candidate: Candidate,
}

/// What the sweep can translate, with what it needs to write it back.
enum Candidate {
    Category { id: i64, name: String, description: Option<String>, translations: Value },
    CategoryDraft { id: i64, draft: Value, fields: CategoryFields },
    MenuItem { id: i64, name: String, description: Option<String>, translations: Value, options: Vec<OptionRow> },
    MenuItemDraft { id: i64, draft: Value, fields: MenuItemFields },
    Rental { id: i64, name: String, description: Option<String>, translations: Value },
}

#[derive(Clone)]
struct OptionRow {
    id: i64,
    label: String,
    translations: Value,
}

fn text_unit(name: &str, description: Option<&str>, translations: &Value) -> Unit {
    Unit::new(translation::english(&[("name", Some(name)), ("description", description)]), translations.clone())
}

impl Candidate {
    fn units(&self) -> Vec<Unit> {
        match self {
            Self::Category { name, description, translations, .. }
            | Self::Rental { name, description, translations, .. } => {
                vec![text_unit(name, description.as_deref(), translations)]
            }
            Self::CategoryDraft { fields, .. } => vec![fields.unit()],
            Self::MenuItem { name, description, translations, options, .. } => {
                let mut units = vec![text_unit(name, description.as_deref(), translations)];
                units.extend(options.iter().map(|option| {
                    Unit::new(translation::english(&[("label", Some(&option.label))]), option.translations.clone())
                }));
                units
            }
            Self::MenuItemDraft { fields, .. } => fields.units(),
        }
    }

    /// Writes the filled units back if nothing changed since they were read.
    async fn write(&self, pool: &PgPool, units: Vec<Unit>) -> Result<bool, sqlx::Error> {
        let affected = match self {
            Self::Category { id, name, description, translations } => {
                sqlx::query!(
                    r#"
                    UPDATE categories SET translations = $2
                    WHERE id = $1 AND deleted_at IS NULL AND translations = $3
                      AND name = $4 AND description IS NOT DISTINCT FROM $5
                    "#,
                    id,
                    units[0].translations,
                    translations,
                    name,
                    description.as_deref(),
                )
                .execute(pool)
                .await?
                .rows_affected()
            }
            Self::CategoryDraft { id, draft, fields } => {
                let mut fields = fields.clone();
                fields.translations = units[0].translations.clone();
                sqlx::query!(
                    "UPDATE categories SET draft = $2 WHERE id = $1 AND draft = $3",
                    id,
                    serde_json::to_value(&fields).expect("serializable"),
                    draft,
                )
                .execute(pool)
                .await?
                .rows_affected()
            }
            Self::Rental { id, name, description, translations } => {
                sqlx::query!(
                    r#"
                    UPDATE rentals SET translations = $2
                    WHERE id = $1 AND deleted_at IS NULL AND translations = $3
                      AND name = $4 AND description IS NOT DISTINCT FROM $5
                    "#,
                    id,
                    units[0].translations,
                    translations,
                    name,
                    description.as_deref(),
                )
                .execute(pool)
                .await?
                .rows_affected()
            }
            Self::MenuItemDraft { id, draft, fields } => {
                let mut fields = fields.clone();
                fields.set_units(units);
                sqlx::query!(
                    "UPDATE menu_items SET draft = $2 WHERE id = $1 AND draft = $3",
                    id,
                    serde_json::to_value(&fields).expect("serializable"),
                    draft,
                )
                .execute(pool)
                .await?
                .rows_affected()
            }
            Self::MenuItem { id, name, description, translations, options } => {
                let mut units = units.into_iter();
                let item = units.next().expect("the item is the first unit");
                let mut tx = pool.begin().await?;
                let affected = sqlx::query!(
                    r#"
                    UPDATE menu_items SET translations = $2
                    WHERE id = $1 AND deleted_at IS NULL AND translations = $3
                      AND name = $4 AND description IS NOT DISTINCT FROM $5
                    "#,
                    id,
                    item.translations,
                    translations,
                    name,
                    description.as_deref(),
                )
                .execute(&mut *tx)
                .await?
                .rows_affected();
                for (option, unit) in options.iter().zip(units) {
                    sqlx::query!(
                        r#"
                        UPDATE menu_item_price_options SET translations = $2
                        WHERE id = $1 AND deleted_at IS NULL AND translations = $3 AND label = $4
                        "#,
                        option.id,
                        unit.translations,
                        option.translations,
                        option.label,
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                affected
            }
        };
        Ok(affected > 0)
    }
}

async fn load_candidates(pool: &PgPool) -> Result<Vec<Entry>, sqlx::Error> {
    let mut candidates = Vec::new();

    for row in sqlx::query!(
        r#"
        SELECT id, name, description, translations, draft, published_at IS NOT NULL as "published!"
        FROM categories WHERE deleted_at IS NULL ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?
    {
        let entry = |sweepable, candidate| Entry { kind: "category", id: row.id, sweepable, candidate };
        if let Some(draft) = row.draft
            && let Ok(fields) = serde_json::from_value::<CategoryFields>(draft.clone())
        {
            candidates.push(entry(false, Candidate::CategoryDraft { id: row.id, draft, fields }));
        }
        candidates.push(entry(row.published, Candidate::Category {
            id: row.id,
            name: row.name,
            description: row.description,
            translations: row.translations,
        }));
    }

    let options = sqlx::query!(
        r#"
        SELECT id, menu_item_id, label, translations FROM menu_item_price_options
        WHERE deleted_at IS NULL ORDER BY menu_item_id, display_order, id
        "#
    )
    .fetch_all(pool)
    .await?;
    for row in sqlx::query!(
        r#"
        SELECT id, name, description, translations, draft, published_at IS NOT NULL as "published!"
        FROM menu_items WHERE deleted_at IS NULL ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?
    {
        let entry = |sweepable, candidate| Entry { kind: "menu_item", id: row.id, sweepable, candidate };
        if let Some(draft) = row.draft
            && let Ok(fields) = serde_json::from_value::<MenuItemFields>(draft.clone())
        {
            candidates.push(entry(false, Candidate::MenuItemDraft { id: row.id, draft, fields }));
        }
        candidates.push(entry(row.published, Candidate::MenuItem {
            id: row.id,
            name: row.name,
            description: row.description,
            translations: row.translations,
            options: options
                .iter()
                .filter(|option| option.menu_item_id == row.id)
                .map(|option| OptionRow {
                    id: option.id,
                    label: option.label.clone(),
                    translations: option.translations.clone(),
                })
                .collect(),
        }));
    }

    for row in sqlx::query!(
        "SELECT id, name, description, translations FROM rentals WHERE deleted_at IS NULL ORDER BY id"
    )
    .fetch_all(pool)
    .await?
    {
        candidates.push(Entry {
            kind: "rental",
            id: row.id,
            // Rentals save live, with a button.
            sweepable: true,
            candidate: Candidate::Rental {
                id: row.id,
                name: row.name,
                description: row.description,
                translations: row.translations,
            },
        });
    }

    Ok(candidates)
}

/// Live rows still waiting for DeepL, and the characters a full fill would
/// send. Drafts are counted when they're published.
pub async fn pending(pool: &PgPool, keep_version: i32) -> Result<(usize, u64), sqlx::Error> {
    let candidates = load_candidates(pool).await?;
    let chars: Vec<u64> = candidates
        .iter()
        .filter(|entry| entry.sweepable)
        .map(|entry| entry.candidate.units().iter().map(|unit| unit.pending_chars(keep_version)).sum())
        .filter(|chars| *chars > 0)
        .collect();
    Ok((chars.len(), chars.iter().sum()))
}

/// One run: returns the characters sent.
pub async fn sweep_once(pool: &PgPool, config: &TranslationConfig) -> Result<u64, sqlx::Error> {
    let settings: TranslationSettings = translation::load_settings(pool).await?;
    if !config.is_configured() || !settings.auto_translate {
        return Ok(0);
    }
    let mut sent = 0;
    let mut written = 0;
    for Entry { candidate, sweepable, .. } in load_candidates(pool).await? {
        if !sweepable {
            continue;
        }
        let mut units = candidate.units();
        let chars: u64 = units.iter().map(|unit| unit.pending_chars(settings.keep_version)).sum();
        if chars == 0 {
            continue;
        }
        if sent + chars > SWEEP_CHAR_CAP {
            break;
        }
        let (used, notice) = config.fill(&settings, &mut units, SWEEP_CHAR_CAP - sent).await;
        sent += used;
        // A failure still writes back: the attempt count is what parks a
        // row after MAX_ATTEMPTS (Decision 20).
        if (used > 0 || notice == Some(Notice::Failed)) && candidate.write(pool, units).await? {
            written += 1;
        }
        if matches!(notice, Some(Notice::Paused | Notice::Disabled | Notice::NotConfigured | Notice::UsageUnknown)) {
            tracing::info!(?notice, "translation sweep stopped");
            break;
        }
    }
    if sent > 0 || written > 0 {
        tracing::info!(chars = sent, rows = written, "translation sweep");
    }
    Ok(sent)
}

/// Decision 2: just before a menu item or category goes live, translate
/// what it's missing — its pending draft if it has one, else its own row.
/// One DeepL round per published change instead of one per autosave.
pub async fn translate_for_publish(
    pool: &PgPool,
    config: &TranslationConfig,
    kind: &str,
    id: i64,
) -> Result<Option<Notice>, sqlx::Error> {
    let settings = translation::load_settings(pool).await?;
    // A row's draft is listed before its live version; the draft is what
    // publish applies.
    let Some(Entry { candidate, .. }) =
        load_candidates(pool).await?.into_iter().find(|entry| entry.kind == kind && entry.id == id)
    else {
        return Ok(None);
    };
    let mut units = candidate.units();
    let (used, notice) = config.fill(&settings, &mut units, translation::SAVE_CHAR_CAP).await;
    if used > 0 || notice == Some(Notice::Failed) {
        candidate.write(pool, units).await?;
    }
    Ok(notice)
}

pub fn spawn_translation_sweeper(pool: PgPool, config: TranslationConfig) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SWEEP_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if let Err(error) = sweep_once(&pool, &config).await {
                tracing::error!(%error, "translation sweep failed");
            }
        }
    });
}
