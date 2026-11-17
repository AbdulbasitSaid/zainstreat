-- Phase 31: events, plates, bookings and past-event photos
-- (requirement.md Decisions 1-15). A booking with meals is paid through a
-- normal order with delivery_type 'event'; a booking without meals has none.
CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    -- {"en": {"title", "summary", "story", "terms", "recap"}, "nl": {...}, "fr": {...}};
    -- English is required, nl/fr fall back to it (Decision 15).
    content JSONB NOT NULL CHECK (content ? 'en'),
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ,
    area TEXT NOT NULL,
    -- Only ever shown to confirmed guests (Decision 10).
    address TEXT NOT NULL,
    capacity INTEGER NOT NULL CHECK (capacity > 0),
    max_party_size INTEGER NOT NULL DEFAULT 6 CHECK (max_party_size > 0),
    meal_orders_close_at TIMESTAMPTZ,
    rsvp_closes_at TIMESTAMPTZ,
    payment_hold_hours INTEGER NOT NULL DEFAULT 48 CHECK (payment_hold_hours > 0),
    cover_image_url TEXT,
    published_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE event_plates (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    -- {"en": {"name", "description"}, ...}
    content JSONB NOT NULL CHECK (content ? 'en'),
    price NUMERIC(10, 2) NOT NULL CHECK (price >= 0),
    is_available BOOLEAN NOT NULL DEFAULT true
);
CREATE INDEX event_plates_event ON event_plates (event_id, position);

CREATE TABLE event_photos (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    image_url TEXT NOT NULL,
    caption TEXT
);
CREATE INDEX event_photos_event ON event_photos (event_id, position);

CREATE TABLE event_bookings (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
    public_token UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    status TEXT NOT NULL CHECK (status IN
        ('waitlisted', 'awaiting_payment', 'confirmed', 'cancelled')),
    cancel_reason TEXT CHECK (cancel_reason IN ('expired', 'admin', 'guest')),
    customer_name TEXT NOT NULL,
    customer_email TEXT NOT NULL,
    customer_phone TEXT NOT NULL,
    party_size INTEGER NOT NULL CHECK (party_size > 0),
    notes TEXT,
    locale TEXT NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'nl', 'fr')),
    -- The latest order for this booking; an offer after a release makes a new one.
    order_id BIGINT UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
    hold_expires_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT event_bookings_cancel_reason
        CHECK ((status = 'cancelled') = (cancel_reason IS NOT NULL))
);
CREATE INDEX event_bookings_event_status ON event_bookings (event_id, status);
CREATE INDEX event_bookings_hold ON event_bookings (hold_expires_at)
    WHERE status = 'awaiting_payment';

-- One row per guest; plate_id NULL means attending without a meal. The
-- plate's name and price are snapshots (mission.md: historical accuracy).
CREATE TABLE event_booking_guests (
    id BIGSERIAL PRIMARY KEY,
    booking_id BIGINT NOT NULL REFERENCES event_bookings(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    plate_id BIGINT REFERENCES event_plates(id) ON DELETE SET NULL,
    plate_name TEXT,
    unit_price NUMERIC(10, 2),
    UNIQUE (booking_id, position),
    CHECK ((plate_name IS NULL) = (unit_price IS NULL))
);

ALTER TABLE orders
    DROP CONSTRAINT orders_delivery_type_check,
    ADD CONSTRAINT orders_delivery_type_check
        CHECK (delivery_type IN ('pickup', 'delivery', 'on_site', 'event'));
