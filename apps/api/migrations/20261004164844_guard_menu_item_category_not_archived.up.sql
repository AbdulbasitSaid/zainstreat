CREATE FUNCTION guard_menu_item_category_not_archived() RETURNS trigger AS $$
DECLARE
    category_deleted_at timestamptz;
BEGIN
    SELECT deleted_at INTO category_deleted_at
    FROM categories
    WHERE id = NEW.category_id
    FOR SHARE;

    IF category_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION 'menu item category % is archived', NEW.category_id
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER menu_items_category_not_archived
    BEFORE INSERT OR UPDATE OF category_id ON menu_items
    FOR EACH ROW
    EXECUTE FUNCTION guard_menu_item_category_not_archived();
