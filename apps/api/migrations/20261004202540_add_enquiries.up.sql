CREATE TABLE catering_enquiries (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT NOT NULL,
    event_type TEXT NOT NULL
        CHECK (event_type IN ('wedding', 'birthday', 'corporate', 'family_gathering', 'outdoor_event', 'other')),
    event_date DATE NOT NULL,
    guest_count INTEGER NOT NULL CHECK (guest_count > 0),
    location TEXT NOT NULL,
    services_required TEXT[] NOT NULL,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE contact_messages (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT NOT NULL,
    subject TEXT NOT NULL
        CHECK (subject IN ('general', 'catering', 'event_rental', 'menu', 'order', 'other')),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
