-- Fails if any 'fr' orders or enquiries exist; a down migration is for dev
-- databases.
ALTER TABLE orders
    DROP CONSTRAINT orders_locale_check,
    ADD CONSTRAINT orders_locale_check CHECK (locale IN ('en', 'nl'));

ALTER TABLE catering_enquiries
    DROP CONSTRAINT catering_enquiries_locale_check,
    ADD CONSTRAINT catering_enquiries_locale_check CHECK (locale IN ('en', 'nl'));
