DROP TABLE event_booking_guests;
DROP TABLE event_bookings;
DROP TABLE event_photos;
DROP TABLE event_plates;
DROP TABLE events;

DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE delivery_type = 'event');
DELETE FROM email_deliveries WHERE order_id IN (SELECT id FROM orders WHERE delivery_type = 'event');
DELETE FROM orders WHERE delivery_type = 'event';

ALTER TABLE orders
    DROP CONSTRAINT orders_delivery_type_check,
    ADD CONSTRAINT orders_delivery_type_check
        CHECK (delivery_type IN ('pickup', 'delivery', 'on_site'));
