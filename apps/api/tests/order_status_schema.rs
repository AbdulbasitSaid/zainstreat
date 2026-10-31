use std::collections::BTreeSet;

use api::order_status::ORDER_STATUSES;
use sqlx::PgPool;

/// Phase 20 open risk 4: `ORDER_STATUSES` and the `orders.status` CHECK are
/// kept in step by hand. This reads the constraint back from Postgres so a
/// migration that adds or drops a status without touching the Rust list (or
/// the other way round) fails here instead of at runtime.
#[sqlx::test]
async fn order_statuses_match_the_check_constraint(pool: PgPool) {
    let definition = sqlx::query_scalar!(
        r#"
        SELECT pg_get_constraintdef(oid) as "definition!"
        FROM pg_constraint
        WHERE conname = 'orders_status_check'
        "#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // e.g. CHECK ((status = ANY (ARRAY['awaiting_payment'::text, ...])))
    let in_check: BTreeSet<&str> = definition.split('\'').skip(1).step_by(2).collect();
    let in_rust: BTreeSet<&str> = ORDER_STATUSES.into_iter().collect();

    assert_eq!(in_check, in_rust, "constraint: {definition}");
}
