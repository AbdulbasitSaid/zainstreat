# Phase 4 — Data Model: Implementation Plan

Numbered, dependency-ordered task groups. Each group should be completed and
sanity-checked before moving to the next.

## Group 0 — Branch

0.1. Create branch `2026-10-02-phase-4-data-model` off `master` (done).

## Group 1 — Cargo dependencies (`apps/api`)

1.1. `apps/api/Cargo.toml` — add to `[dependencies]`:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "macros", "migrate", "chrono"] }
chrono = { version = "0.4", features = ["serde"] }
```

1.2. Install `sqlx-cli` locally (not a project dependency — a cargo
     subcommand used to generate migrations and the offline query cache):
```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## Group 2 — Migration: initial schema

Depends on: Group 1 (for `sqlx migrate add` conventions; the SQL itself has
no Rust dependency).

2.1. From `apps/api`, with `DATABASE_URL` pointing at the dev Postgres
     (see Group 4 for where that env var comes from locally):
```bash
sqlx migrate add -r initial_schema
```
     This creates `apps/api/migrations/<timestamp>_initial_schema.up.sql`
     and `..._initial_schema.down.sql` (reversible migration — `-r`).

2.2. `..._initial_schema.up.sql`:
```sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE categories (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE menu_items (
    id BIGSERIAL PRIMARY KEY,
    category_id BIGINT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    description TEXT,
    price NUMERIC(10, 2) NOT NULL,
    image_url TEXT,
    is_available BOOLEAN NOT NULL DEFAULT true,
    is_featured BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
CREATE INDEX idx_menu_items_category_id ON menu_items(category_id);

CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    customer_name TEXT NOT NULL,
    customer_email TEXT NOT NULL,
    customer_phone TEXT NOT NULL,
    delivery_type TEXT NOT NULL CHECK (delivery_type IN ('pickup', 'delivery')),
    delivery_address TEXT,
    notes TEXT,
    subtotal NUMERIC(10, 2) NOT NULL,
    delivery_fee NUMERIC(10, 2) NOT NULL DEFAULT 0,
    total NUMERIC(10, 2) NOT NULL,
    status TEXT NOT NULL DEFAULT 'new'
        CHECK (status IN ('new', 'confirmed', 'preparing', 'ready', 'completed', 'cancelled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_orders_status ON orders(status);

CREATE TABLE order_items (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    menu_item_id BIGINT REFERENCES menu_items(id) ON DELETE SET NULL,
    item_name TEXT NOT NULL,
    unit_price NUMERIC(10, 2) NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    subtotal NUMERIC(10, 2) NOT NULL
);
CREATE INDEX idx_order_items_order_id ON order_items(order_id);
```

2.3. `..._initial_schema.down.sql`:
```sql
DROP TABLE order_items;
DROP TABLE orders;
DROP TABLE menu_items;
DROP TABLE categories;
DROP TABLE users;
```

## Group 3 — Wire migrations + DB pool into the API

Depends on: Groups 1, 2.

3.1. `apps/api/src/main.rs` — on startup, before binding the listener:
```rust
let database_url = std::env::var("DATABASE_URL")
    .expect("DATABASE_URL must be set");

let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await
    .expect("failed to connect to Postgres");

sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .expect("failed to run database migrations");
```
     Exit non-zero (via the `.expect` panics above) if either the initial
     connection or a migration fails — a container that can't reach its
     database or has a broken migration should not come up looking
     healthy.

3.2. Make the pool available to the `/health` handler (e.g. `axum::Router::with_state(pool)`
     or an `Extension`), and update it to prove DB connectivity:
```rust
async fn health(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "db_unreachable" })),
        ),
    }
}
```
     (Adjust imports/signature to match whatever axum 0.8 idiom the rest
     of `main.rs` already uses — this is illustrative, not copy-paste
     exact.)

## Group 4 — `DATABASE_URL` wiring

Depends on: Group 3.

4.1. `docker-compose.yml` — add to the `api` service's `environment:`
     block:
```yaml
DATABASE_URL: postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
```

4.2. `.env.example` — no new variable needed (reuses
     `POSTGRES_USER`/`POSTGRES_PASSWORD`/`POSTGRES_DB`, already present).
     Add a one-line comment above the Postgres block noting `DATABASE_URL`
     for the `api` container is composed from these three in
     `docker-compose.yml`, so a local `sqlx`/`psql` session run *outside*
     Docker (e.g. for `cargo sqlx prepare`) should export the equivalent
     `postgres://...@localhost:${POSTGRES_PORT:-5432}/...` by hand.

## Group 5 — Offline query cache for CI

Depends on: Group 3 (needs at least one real `sqlx::query`/`query!` call
to generate a non-empty cache — the `/health` check from 3.2 covers this
for now; Phase 6 will add more).

5.1. With the dev stack up (`docker compose up`) and a local
     `DATABASE_URL` exported matching 4.2's note, run from `apps/api`:
```bash
cargo sqlx prepare
```
     Commit the resulting `apps/api/.sqlx/` directory — do **not**
     gitignore it.

5.2. `apps/api/Dockerfile` — add before the `cargo build --release` line
     in the build stage:
```dockerfile
ENV SQLX_OFFLINE=true
```

## Group 6 — Seed script

Depends on: Group 2.

6.1. `apps/api/seed.sql` — idempotent (safe to run more than once):
```sql
INSERT INTO categories (name, description) VALUES
    ('Rice Dishes', 'Classic rice-based meals'),
    ('Snacks', 'Light bites and finger foods'),
    ('Drinks', 'Beverages to go with your meal')
ON CONFLICT DO NOTHING;

INSERT INTO menu_items (category_id, name, description, price, image_url, is_available, is_featured)
SELECT c.id, v.name, v.description, v.price, v.image_url, true, v.is_featured
FROM (VALUES
    ('Rice Dishes', 'Jollof Rice', 'Smoky party-style jollof rice', 6000.00, 'https://placehold.co/600x400?text=Jollof+Rice', true),
    ('Rice Dishes', 'Fried Rice', 'Vegetable fried rice', 6000.00, 'https://placehold.co/600x400?text=Fried+Rice', false),
    ('Snacks', 'Chicken Suya', 'Grilled spiced chicken skewers', 3500.00, 'https://placehold.co/600x400?text=Chicken+Suya', true),
    ('Snacks', 'Puff Puff', 'Sweet fried dough balls (6 pcs)', 1500.00, 'https://placehold.co/600x400?text=Puff+Puff', false),
    ('Snacks', 'Meat Pie', 'Savory pastry with minced meat filling', 1000.00, 'https://placehold.co/600x400?text=Meat+Pie', false),
    ('Snacks', 'Chin Chin', 'Crunchy sweet fried snack (pack)', 1500.00, 'https://placehold.co/600x400?text=Chin+Chin', false)
) AS v(category_name, name, description, price, image_url, is_featured)
JOIN categories c ON c.name = v.category_name
ON CONFLICT DO NOTHING;
```
     (`ON CONFLICT DO NOTHING` needs a unique constraint to target — if
     `menu_items.name` isn't unique, drop that clause from the second
     insert and instead guard re-runs with a `WHERE NOT EXISTS (...)` or
     simply document that `seed.sql` is meant to run once against a fresh
     database, not repeatedly. Decide whichever is simplest at
     implementation time; either is fine for a dev-only fixture script.)

## Group 7 — Docs

Depends on: Groups 2, 5, 6. (`specs/tech-stack.md`'s "Backend API" section
already documents the migration/offline-mode decisions as of planning —
nothing further needed there.)

7.1. `docs/local-development.md` — add a "Database" section:
     - How migrations run (automatically, on API container startup — no
       manual step for normal `docker compose up`).
     - How to add a new migration: `cd apps/api && sqlx migrate add -r <name>`.
     - How to regenerate the offline query cache after changing a query:
       `cd apps/api && cargo sqlx prepare` (requires `DATABASE_URL`
       exported and the dev stack running), then commit `.sqlx/`.
     - How to load sample data: `docker compose exec -T postgres psql -U
       $POSTGRES_USER -d $POSTGRES_DB < apps/api/seed.sql` (or the
       equivalent run locally against `localhost:$POSTGRES_PORT`).

## Group 8 — Verification

See `validation.md` for the full pass/fail checklist.
