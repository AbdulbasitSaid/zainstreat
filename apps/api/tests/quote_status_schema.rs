use std::collections::BTreeSet;

use api::quote_status::QUOTE_STATUSES;
use sqlx::PgPool;

/// Same guard as `order_status_schema.rs`, for `catering_quotes.status`.
#[sqlx::test]
async fn quote_statuses_match_the_check_constraint(pool: PgPool) {
    let definition = sqlx::query_scalar!(
        r#"
        SELECT pg_get_constraintdef(oid) as "definition!"
        FROM pg_constraint
        WHERE conname = 'catering_quotes_status_check'
        "#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let in_check: BTreeSet<&str> = definition.split('\'').skip(1).step_by(2).collect();
    let in_rust: BTreeSet<&str> = QUOTE_STATUSES.into_iter().collect();

    assert_eq!(in_check, in_rust, "constraint: {definition}");
}
