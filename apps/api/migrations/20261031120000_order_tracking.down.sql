ALTER TABLE orders
    DROP COLUMN locale,
    DROP COLUMN ready_at,
    DROP COLUMN completed_at,
    DROP COLUMN cancelled_at,
    DROP COLUMN completed_by;
