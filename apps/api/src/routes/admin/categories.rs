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
}

struct CategoryRow {
    id: i64,
    name: String,
    description: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<CategoryRow> for AdminCategoryResponse {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            is_archived: row.deleted_at.is_some(),
            updated_at: row.updated_at,
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
               updated_at as "updated_at: DateTime<Utc>"
        FROM categories ORDER BY display_order, id
        "#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Disambiguates a 0-row conditional UPDATE: row gone entirely -> 404,
/// row still there but `updated_at` moved on -> 409 with the live row.
async fn category_or_conflict(pool: &PgPool, id: i64) -> AppError {
    match sqlx::query_as!(
        CategoryRow,
        r#"
        SELECT id, name, description,
               deleted_at as "deleted_at: DateTime<Utc>",
               updated_at as "updated_at: DateTime<Utc>"
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
pub struct UpsertCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ArchiveRequest {
    pub updated_at: DateTime<Utc>,
}

fn validate_category(payload: &UpsertCategoryRequest) -> Result<(), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation(vec![FieldError {
            field: "name".into(),
            message: "required".into(),
        }]));
    }
    Ok(())
}

pub async fn create_category(
    State(pool): State<PgPool>,
    Json(payload): Json<UpsertCategoryRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload)?;

    // Appended last (requirement.md Decision 8) — no reorder UI this phase.
    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        INSERT INTO categories (name, description, display_order)
        VALUES ($1, $2, (SELECT COALESCE(MAX(display_order), -1) + 1 FROM categories))
        RETURNING id, name, description,
                  deleted_at as "deleted_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>"
        "#,
        payload.name.trim(),
        payload.description,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(row.into()))
}

pub async fn update_category(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<UpsertCategoryRequest>,
) -> Result<Json<AdminCategoryResponse>, AppError> {
    validate_category(&payload)?;

    let row = sqlx::query_as!(
        CategoryRow,
        r#"
        UPDATE categories SET name = $3, description = $4, updated_at = now()
        WHERE id = $1 AND updated_at = $2
        RETURNING id, name, description,
                  deleted_at as "deleted_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>"
        "#,
        id,
        chrono_to_offset(payload.updated_at),
        payload.name.trim(),
        payload.description,
    )
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(row) => Ok(Json(row.into())),
        None => Err(category_or_conflict(&pool, id).await),
    }
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
        UPDATE categories SET deleted_at = now(), updated_at = now()
        WHERE id = $1 AND updated_at = $2
        RETURNING id, name, description,
                  deleted_at as "deleted_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>"
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
