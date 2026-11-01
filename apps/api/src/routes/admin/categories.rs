use crate::chrono_to_offset;
use crate::error::{AppError, BlockingMenuItem, FieldError};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct AdminCategoryResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_archived: bool,
    pub updated_at: DateTime<Utc>,
    /// Phase 25 Decision 15: new categories start unpublished.
    pub is_published: bool,
    pub has_draft: bool,
    pub draft: Option<CategoryFields>,
    pub draft_updated_at: Option<DateTime<Utc>>,
}

pub(crate) struct CategoryRow {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) deleted_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) draft: Option<serde_json::Value>,
    pub(crate) draft_updated_at: Option<DateTime<Utc>>,
    pub(crate) published_at: Option<DateTime<Utc>>,
}

impl From<CategoryRow> for AdminCategoryResponse {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            is_archived: row.deleted_at.is_some(),
            updated_at: row.updated_at,
            is_published: row.published_at.is_some(),
            has_draft: row.draft.is_some(),
            draft: row.draft.and_then(|draft| parse_category_draft(row.id, draft)),
            draft_updated_at: row.draft_updated_at,
        }
    }
}

/// A category's editable content: the create body, the draft-save body and
/// the shape stored in `categories.draft` (Phase 25 Decision 15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryFields {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveCategoryDraftRequest {
    #[serde(flatten)]
    pub fields: CategoryFields,
    pub draft_updated_at: Option<DateTime<Utc>>,
}

pub(crate) fn parse_category_draft(id: i64, draft: serde_json::Value) -> Option<CategoryFields> {
    match serde_json::from_value(draft) {
        Ok(fields) => Some(fields),
        Err(err) => {
            tracing::error!(%err, id, "unreadable category draft");
            None
        }
    }
}

pub async fn list_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AdminCategoryResponse>>, AppError> {
    let rows = sqlx::query_as!(
        CategoryRow,
        r#"
        SELECT id, name, description,
               deleted_at as "deleted_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>",
               draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM categories ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Disambiguates a 0-row conditional UPDATE: row gone entirely -> 404,
/// row still there but `updated_at` moved on -> 409 with the live row.
pub(crate) async fn category_or_conflict(pool: &PgPool, id: i64) -> AppError {
    match sqlx::query_as!(
        CategoryRow,
        r#"
        SELECT id, name, description,
               deleted_at as "deleted_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>",
               draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM categories WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(row)) => {
            let current: AdminCategoryResponse = row.into();
            AppError::Conflict(serde_json::to_value(current).expect("serializable"))
        }
        Ok(None) => AppError::NotFound,
        Err(err) => AppError::Database(err),
    }
}

#[derive(Debug, Deserialize)]
pub struct ArchiveRequest {
    pub updated_at: DateTime<Utc>,
}

pub(crate) fn validate_category(payload: &CategoryFields) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation(vec![FieldError {
            field: "name".into(),
            message: "required".into(),
        }]));
    }
    Ok(())
}

/// New categories start unpublished (Phase 25 Decision 15).
pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<CategoryFields>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload)?;

    // Appended last (requirement.md Decision 8) — no reorder UI this phase.
    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        INSERT INTO categories (name, description, display_order, published_at)
        VALUES ($1, $2, (SELECT COALESCE(MAX(display_order), -1) + 1 FROM categories), NULL)
        RETURNING id, name, description,
                  deleted_at as "deleted_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>",
                  draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
                  published_at as "published_at: DateTime<Utc>"
        "#,
        payload.name.trim(),
        payload.description,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(row.into()))
}

/// Same model as `save_menu_item_draft`: a published category's edit goes
/// to `draft`, an unpublished one's to the live columns.
pub async fn save_category_draft(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<SaveCategoryDraftRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload.fields)?;

    let mut tx = pool.begin().await?;
    let row = sqlx::query!(
        r#"
        SELECT deleted_at as "deleted_at: DateTime<Utc>",
               draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM categories WHERE id = $1
        FOR UPDATE
        "#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.deleted_at.is_some() {
        return Err(AppError::NotFound);
    }
    if row.draft_updated_at != payload.draft_updated_at {
        tx.rollback().await?;
        return Err(category_or_conflict(&pool, id).await);
    }

    if row.published_at.is_none() {
        apply_category_fields(&mut tx, id, &payload.fields).await?;
        sqlx::query!("UPDATE categories SET draft_updated_at = now() WHERE id = $1", id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query!(
            "UPDATE categories SET draft = $2, draft_updated_at = now() WHERE id = $1",
            id,
            serde_json::to_value(&payload.fields).expect("serializable"),
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    load_category(&pool, id).await.map(Json)
}

pub async fn get_category(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    load_category(&pool, id).await.map(Json)
}

pub(crate) async fn apply_category_fields(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: i64,
    fields: &CategoryFields,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE categories SET name = $2, description = $3, updated_at = now() WHERE id = $1",
        id,
        fields.name.trim(),
        fields.description,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn load_category(pool: &PgPool, id: i64) -> Result<AdminCategoryResponse, AppError> {
    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        SELECT id, name, description,
               deleted_at as "deleted_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>",
               draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
               published_at as "published_at: DateTime<Utc>"
        FROM categories WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(row.into())
}

pub async fn archive_category(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ArchiveRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    let blocking = sqlx::query_as!(
        BlockingMenuItem,
        r#"SELECT id, name FROM menu_items WHERE category_id = $1 AND deleted_at IS NULL"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    if !blocking.is_empty() {
        return Err(AppError::CategoryHasActiveItems(blocking));
    }

    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        UPDATE categories SET deleted_at = now(), updated_at = now(), draft = NULL, draft_updated_at = NULL
        WHERE id = $1 AND updated_at = $2
        RETURNING id, name, description,
                  deleted_at as "deleted_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>",
                  draft, draft_updated_at as "draft_updated_at: DateTime<Utc>",
                  published_at as "published_at: DateTime<Utc>"
        "#,
        id,
        chrono_to_offset(payload.updated_at),
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Json(row.into())),
        None => Err(category_or_conflict(&pool, id).await),
    }
}
