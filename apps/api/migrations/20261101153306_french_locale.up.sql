-- Phase 24: French as a third customer locale (requirement.md Decision 6).
ALTER TABLE orders
    DROP CONSTRAINT orders_locale_check,
    ADD CONSTRAINT orders_locale_check CHECK (locale IN ('en', 'nl', 'fr'));

ALTER TABLE catering_enquiries
    DROP CONSTRAINT catering_enquiries_locale_check,
    ADD CONSTRAINT catering_enquiries_locale_check CHECK (locale IN ('en', 'nl', 'fr'));
