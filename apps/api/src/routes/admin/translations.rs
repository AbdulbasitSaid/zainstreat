//! Phase 38: the Translations section of Settings (requirement.md
//! Decisions 6, 20): the DeepL usage meter, the stop line, the on/off
//! switch and the "Words to keep" list.

use crate::chrono_to_offset;
use crate::error::{AppError, FieldError};
use crate::translation::{self, TranslationConfig, Usage};
use crate::translation_sweep;
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const MAX_KEEP_WORDS: usize = 100;
const MAX_KEEP_WORD_CHARS: usize = 40;

#[derive(Debug, Serialize)]
pub struct TranslationStatusResponse {
    /// `DEEPL_API_KEY` is set.
    pub configured: bool,
    pub auto_translate: bool,
    pub stop_at_percent: i32,
    pub keep_words: Vec<String>,
    pub updated_at: DateTime<Utc>,
    /// DeepL's own count; null when it can't be read.
    pub usage: Option<Usage>,
    /// The count past which nothing more is sent.
    pub stop_line: Option<u64>,
    /// Rows waiting for DeepL and the characters they'd use.
    pub pending_rows: usize,
    pub pending_chars: u64,
    /// What changing the keep list would re-translate (the confirmation).
    pub keep_change_chars: u64,
}

pub async fn get_status(
    State(pool): State<PgPool>,
    State(translator): State<TranslationConfig>,
) -> Result<Json<TranslationStatusResponse>, AppError> {
    let settings = translation::load_settings(&pool).await?;
    let usage = if translator.is_configured() { translator.usage().await } else { None };
    let (pending_rows, pending_chars) = translation_sweep::pending(&pool, settings.keep_version).await?;
    let (_, keep_change_chars) = translation_sweep::pending(&pool, settings.keep_version + 1).await?;
    Ok(Json(TranslationStatusResponse {
        configured: translator.is_configured(),
        auto_translate: settings.auto_translate,
        stop_at_percent: settings.stop_at_percent,
        stop_line: usage.map(|usage| translation::stop_line(usage.limit, settings.stop_at_percent)),
        keep_words: settings.keep_words,
        updated_at: settings.updated_at,
        usage,
        pending_rows,
        pending_chars,
        keep_change_chars,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SaveSettingsRequest {
    pub auto_translate: bool,
    pub stop_at_percent: i32,
    pub keep_words: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

/// Trimmed, blank lines dropped, duplicates (ignoring case) removed.
fn clean_keep_words(words: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in words.iter().map(|word| word.trim()).filter(|word| !word.is_empty()) {
        if !out.iter().any(|kept| kept.to_lowercase() == word.to_lowercase()) {
            out.push(word.to_string());
        }
    }
    out
}

/// A changed keep list bumps `keep_version`, so every automatic translation
/// is redone by the sweep (Decision 6); the owner's own are kept.
pub async fn save_settings(
    State(pool): State<PgPool>,
    State(translator): State<TranslationConfig>,
    Json(payload): Json<SaveSettingsRequest>,
) -> Result<Json<TranslationStatusResponse>, AppError> {
    let keep_words = clean_keep_words(&payload.keep_words);
    let mut errors = Vec::new();
    if !(10..=100).contains(&payload.stop_at_percent) {
        errors.push(FieldError { field: "stop_at_percent".into(), message: "invalid".into() });
    }
    if keep_words.len() > MAX_KEEP_WORDS || keep_words.iter().any(|word| word.chars().count() > MAX_KEEP_WORD_CHARS) {
        errors.push(FieldError { field: "keep_words".into(), message: "too_long".into() });
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let affected = sqlx::query!(
        r#"
        UPDATE translation_settings
        SET keep_version = keep_version + (keep_words IS DISTINCT FROM $1::text[])::int,
            keep_words = $1, auto_translate = $2, stop_at_percent = $3, updated_at = now()
        WHERE updated_at = $4
        "#,
        &keep_words,
        payload.auto_translate,
        payload.stop_at_percent,
        chrono_to_offset(payload.updated_at),
    )
    .execute(&pool)
    .await?
    .rows_affected();

    let status = get_status(State(pool), State(translator)).await?;
    if affected == 0 {
        return Err(AppError::Conflict(serde_json::to_value(status.0).expect("serializable")));
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_words_are_trimmed_and_deduplicated() {
        let words = vec![" Jollof ".into(), "".into(), "jollof".into(), "Suya".into()];
        assert_eq!(clean_keep_words(&words), vec!["Jollof".to_string(), "Suya".to_string()]);
    }
}
