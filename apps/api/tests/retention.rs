use api::retention::{SweepCounts, sweep};
use sqlx::PgPool;

/// An order `months` months old. Returns its id.
async fn insert_order(pool: &PgPool, status: &str, paid: bool, months: i32) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO orders
            (customer_name, customer_email, customer_phone, delivery_type, delivery_address,
             notes, subtotal, total, status, payment_confirmed_at, created_at)
        VALUES ('Ada', 'ada@example.com', '+31612345678', 'delivery', 'Main St 1', 'ring twice',
                12.50, 12.50, $1,
                CASE WHEN $2 THEN now() - make_interval(months => $3) END,
                now() - make_interval(months => $3))
        RETURNING id
        "#,
    )
    .bind(status)
    .bind(paid)
    .bind(months)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn insert_enquiry(pool: &PgPool, months: i32) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO catering_enquiries
            (name, phone, email, event_type, event_date, guest_count, location, services_required,
             message, created_at)
        VALUES ('Amina', '0612345678', 'amina@example.com', 'wedding', CURRENT_DATE, 80,
                'Utrecht', ARRAY['catering'], 'hello', now() - make_interval(months => $1))
        RETURNING id
        "#,
    )
    .bind(months)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// An event `months` months ago with one free RSVP and one paid booking.
/// Returns (free booking id, paid booking id).
async fn insert_event_bookings(pool: &PgPool, months: i32) -> (i64, i64) {
    let event_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO events (slug, content, starts_at, area, address, capacity, max_party_size)
        VALUES ('past-' || $1, '{"en": {"title": "Past"}}', now() - make_interval(months => $1),
                'Almere', 'Somewhere 1', 20, 6)
        RETURNING id
        "#,
    )
    .bind(months)
    .fetch_one(pool)
    .await
    .unwrap();
    let paid_order = insert_order(pool, "completed", true, months).await;
    let mut ids = Vec::new();
    for order_id in [None, Some(paid_order)] {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO event_bookings
                (event_id, status, customer_name, customer_email, customer_phone, party_size,
                 order_id, created_at)
            VALUES ($1, 'confirmed', 'Bola', 'bola@example.com', '0611111111', 2, $2,
                    now() - make_interval(months => $3))
            RETURNING id
            "#,
        )
        .bind(event_id)
        .bind(order_id)
        .bind(months)
        .fetch_one(pool)
        .await
        .unwrap();
        ids.push(id);
    }
    (ids[0], ids[1])
}

async fn is_anonymised(pool: &PgPool, table: &str, id: i64) -> bool {
    sqlx::query_scalar(&format!(
        "SELECT anonymised_at IS NOT NULL FROM {table} WHERE id = $1"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn sweep_follows_the_retention_table(pool: PgPool) {
    let recent_order = insert_order(&pool, "completed", true, 2).await;
    let old_paid_order = insert_order(&pool, "completed", true, 85).await;
    let year_old_paid_order = insert_order(&pool, "completed", true, 13).await;
    let unpaid_cancelled = insert_order(&pool, "cancelled", false, 13).await;
    let old_enquiry = insert_enquiry(&pool, 13).await;
    let recent_enquiry = insert_enquiry(&pool, 2).await;
    let (free_rsvp, paid_booking) = insert_event_bookings(&pool, 13).await;
    sqlx::query(
        r#"
        INSERT INTO contact_messages (name, email, phone, subject, message, created_at)
        VALUES ('Old', 'o@example.com', '0', 'general', 'hi', now() - interval '13 months'),
               ('New', 'n@example.com', '0', 'general', 'hi', now() - interval '2 months')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO email_deliveries (order_id, kind, recipient, created_at)
        VALUES ($1, 'invoice', 'ada@example.com', now() - interval '100 days'),
               ($1, 'status', 'ada@example.com', now() - interval '10 days')
        "#,
    )
    .bind(recent_order)
    .execute(&pool)
    .await
    .unwrap();
    let old_token: uuid::Uuid = sqlx::query_scalar("SELECT public_token FROM orders WHERE id = $1")
        .bind(old_paid_order)
        .fetch_one(&pool)
        .await
        .unwrap();

    let counts = sweep(&pool).await.unwrap();
    assert_eq!(
        counts,
        SweepCounts {
            orders: 2,
            bookings: 1,
            enquiries: 1,
            contact_messages: 1,
            email_deliveries: 1,
            login_failures: 0,
        }
    );

    assert!(is_anonymised(&pool, "orders", old_paid_order).await);
    assert!(is_anonymised(&pool, "orders", unpaid_cancelled).await);
    assert!(!is_anonymised(&pool, "orders", recent_order).await);
    assert!(!is_anonymised(&pool, "orders", year_old_paid_order).await);
    assert!(is_anonymised(&pool, "catering_enquiries", old_enquiry).await);
    assert!(!is_anonymised(&pool, "catering_enquiries", recent_enquiry).await);
    assert!(is_anonymised(&pool, "event_bookings", free_rsvp).await);
    assert!(!is_anonymised(&pool, "event_bookings", paid_booking).await);

    // Personal fields gone, amounts and status kept, tracking link dead.
    let (name, email, phone, address, notes, total, status, token): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        rust_decimal::Decimal,
        String,
        uuid::Uuid,
    ) = sqlx::query_as(
        "SELECT customer_name, customer_email, customer_phone, delivery_address, notes, total, status, public_token FROM orders WHERE id = $1",
    )
    .bind(old_paid_order)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(name, "Anonymised");
    assert_eq!(email, format!("anonymised+{old_paid_order}@invalid"));
    assert_eq!(phone, "");
    assert_eq!((address, notes), (None, None));
    assert_eq!(total.to_string(), "12.50");
    assert_eq!(status, "completed");
    assert_ne!(token, old_token);

    // Idempotent.
    assert_eq!(sweep(&pool).await.unwrap(), SweepCounts::default());
}
