DROP TABLE privacy_requests;
ALTER TABLE catering_enquiries DROP COLUMN anonymised_at;
ALTER TABLE event_bookings DROP COLUMN anonymised_at;
ALTER TABLE orders DROP COLUMN anonymised_at;
