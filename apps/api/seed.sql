-- Dev-only fixture data: the real Zain's Treat n More menu
-- (Phase 7 — Menu Price Variants). Full replacement of the Phase 4/6
-- placeholder catalogue — the TRUNCATE below makes this script idempotent
-- and safe to re-run, unlike the previous seed.sql which only ever
-- appended.
--
-- Invariant (application-level, not DB-enforced — see the migration's own
-- comment): each menu item below has EITHER a flat `price` (and zero
-- menu_item_price_options rows) OR one-or-more menu_item_price_options
-- rows (and a NULL price) — never both.

TRUNCATE TABLE menu_item_price_options, menu_items, categories
    RESTART IDENTITY CASCADE;

INSERT INTO categories (name, description, display_order) VALUES
    ('Individual Plates', 'Standard MOQ: 10 plates. 5-plate orders: add €2 per plate.', 1),
    ('By The Litre', NULL, 2),
    ('Soups & Stews', NULL, 3),
    ('Bulk Orders', NULL, 4);

-- Flat-price items (price IS NOT NULL, zero price_options rows).
INSERT INTO menu_items (category_id, name, description, price, image_url, is_available, is_featured, display_order)
SELECT c.id, v.name, v.description, v.price, v.image_url, true, v.is_featured, v.display_order
FROM (VALUES
    ('Individual Plates', 'Jollof Rice & Plantain with Chicken or Turkey', 'Smoky, tomato-based jollof rice served with sweet fried plantain and a choice of chicken or turkey — a 1 L portion.', 13.00, '/images/menu/jollof-rice-plantain.jpg', true, 1),
    ('Individual Plates', 'Fried Rice & Plantain with Chicken or Turkey', 'Lightly spiced vegetable fried rice served with sweet fried plantain and a choice of chicken or turkey — a 1 L portion.', 15.00, '/images/menu/fried-rice-plantain.jpg', false, 2),
    ('Individual Plates', 'Ewa Agoyin with Plantain & Fish', 'Soft mashed beans in a rich, peppered palm-oil sauce, served with fried plantain and fish.', 15.00, '/images/menu/ewa-agoyin.jpg', false, 3),
    ('Individual Plates', 'Vegetable Salad', 'A fresh, colourful vegetable salad — a light side to any plate.', 3.00, '/images/menu/vegetable-salad.jpg', false, 4),
    ('Individual Plates', 'White Rice & Beans with Pepper Sauce & Beef', 'Steamed white rice and beans served with a peppery sauce and beef — a 1 L portion.', 15.00, '/images/menu/white-rice-beans-pepper-beef.jpg', false, 5),
    ('Individual Plates', 'White Rice & Beans with Plantain & Assorted Meat Stew', 'Steamed white rice and beans with fried plantain and a hearty assorted meat stew — a 1 L portion.', 20.00, '/images/menu/white-rice-beans-assorted-stew.jpg', false, 6),
    ('Individual Plates', 'Moin Moin with Fish & Egg', 'Steamed savoury bean pudding packed with fish and egg.', 6.00, '/images/menu/moin-moin-fish-egg.jpg', false, 7),
    ('Individual Plates', 'Moin Moin with Egg', 'Steamed savoury bean pudding with egg.', 5.00, '/images/menu/moin-moin-egg.jpg', false, 8),

    ('By The Litre', '5 L Jollof Rice + 5 Pieces of Chicken or Turkey', 'A 5-litre batch of smoky jollof rice with 5 pieces of chicken or turkey — ideal for sharing.', 85.00, '/images/menu/jollof-rice-plantain.jpg', false, 1),
    ('By The Litre', '5 L Fried Rice + 5 Pieces of Chicken or Turkey', 'A 5-litre batch of vegetable fried rice with 5 pieces of chicken or turkey — ideal for sharing.', 105.00, '/images/menu/fried-rice-plantain.jpg', false, 2),
    ('By The Litre', '2 L Ewa Agoyin + 500 ml Agoyin Sauce', 'A 2-litre portion of mashed beans with an extra 500 ml of peppered agoyin sauce on the side.', 65.00, '/images/menu/ewa-agoyin.jpg', false, 3),
    ('By The Litre', '2 L Fried Plantain', 'A generous 2-litre portion of sweet fried plantain.', 45.00, '/images/menu/fried-plantain.jpg', false, 4),
    ('By The Litre', '5 L White Rice + 1.5 L Assorted Meat Stew', 'A 5-litre portion of steamed white rice with 1.5 litres of assorted meat stew.', 75.00, '/images/menu/white-rice-beans-assorted-stew.jpg', false, 5),

    ('Soups & Stews', 'Assorted Meat Stew', 'A hearty, tomato-based stew with a mix of assorted meats — a 2.5 L portion.', 95.00, '/images/menu/assorted-meat-stew.jpg', false, 7),
    ('Soups & Stews', 'Smoked Mackerel Fish Sauce', 'A savoury tomato-based sauce prepared with smoked mackerel fish — a 2.5 L portion.', 80.00, '/images/menu/smoked-mackerel-sauce.jpg', false, 8),
    ('Soups & Stews', 'Pepper Sauce', 'A fiery tomato-and-pepper sauce without protein — a 2 L portion.', 45.00, '/images/menu/pepper-sauce.jpg', false, 9),

    ('Bulk Orders', '50 Pieces of Fried Fish', 'Fifty pieces of golden, crisp fried fish — perfect for large events.', 200.00, '/images/menu/fried-fish.jpg', false, 5),
    ('Bulk Orders', '½ Tray of Vegetable Salad', 'A half tray of fresh vegetable salad — perfect for sharing at events.', 100.00, '/images/menu/vegetable-salad.jpg', false, 6)
) AS v(category_name, name, description, price, image_url, is_featured, display_order)
JOIN categories c ON c.name = v.category_name;

-- Items priced by size/variant (price IS NULL; see menu_item_price_options below).
INSERT INTO menu_items (category_id, name, description, image_url, is_available, is_featured, display_order)
SELECT c.id, v.name, v.description, v.image_url, true, v.is_featured, v.display_order
FROM (VALUES
    ('Soups & Stews', 'Egusi Soup', 'A rich Nigerian soup made from ground melon seeds in palm oil — best paired with a swallow of choice.', '/images/menu/egusi-soup.jpeg', true, 1),
    ('Soups & Stews', 'Efo Riro', 'A vibrant Nigerian vegetable soup simmered in a palm-oil base — best paired with a swallow of choice.', '/images/menu/efo-soup.jpeg', false, 2),
    ('Soups & Stews', 'Ogbono Soup', 'A traditional Nigerian soup thickened with ground ogbono seeds — best paired with a swallow of choice.', '/images/menu/ogbono-soup.jpg', false, 3),
    ('Soups & Stews', 'Ofada Stew', 'A bold, peppery ofada-style stew — best paired with rice or a swallow of choice.', '/images/menu/ofada-stew.jpg', false, 4),
    ('Soups & Stews', 'Chicken Stew', 'A classic tomato-based stew simmered with tender chicken.', '/images/menu/poultry-stew.jpg', false, 5),
    ('Soups & Stews', 'Turkey Stew', 'A classic tomato-based stew simmered with tender turkey.', '/images/menu/poultry-stew.jpg', false, 6),

    ('Bulk Orders', 'Cooler of Jollof Rice', 'A full cooler of smoky jollof rice — perfect for parties and large gatherings.', '/images/menu/jollof-rice-plantain.jpg', true, 1),
    ('Bulk Orders', 'Cooler of Fried Rice', 'A full cooler of vegetable fried rice — perfect for parties and large gatherings.', '/images/menu/fried-rice-plantain.jpg', false, 2),
    ('Bulk Orders', 'Box of Peppered Turkey', 'A box of peppered, pan-fried turkey pieces — great for parties and bulk orders.', '/images/menu/peppered-poultry.jpg', false, 3),
    ('Bulk Orders', 'Box of Peppered Chicken', 'A box of peppered, pan-fried chicken pieces — great for parties and bulk orders.', '/images/menu/peppered-poultry.jpg', false, 4)
) AS v(category_name, name, description, image_url, is_featured, display_order)
JOIN categories c ON c.name = v.category_name;

-- Price options for the 10 variant-priced items above (20 rows total).
INSERT INTO menu_item_price_options (menu_item_id, label, price, display_order)
SELECT m.id, v.label, v.price, v.display_order
FROM (VALUES
    ('Egusi Soup', '2 L', 75.00, 1),
    ('Egusi Soup', '3 L', 95.00, 2),
    ('Efo Riro', '2 L', 90.00, 1),
    ('Efo Riro', '3 L', 105.00, 2),
    ('Ogbono Soup', '2 L', 75.00, 1),
    ('Ogbono Soup', '3 L', 90.00, 2),
    ('Ofada Stew', '2 L', 80.00, 1),
    ('Ofada Stew', '3 L', 95.00, 2),
    ('Chicken Stew', '2.5 L', 65.00, 1),
    ('Chicken Stew', '5 L', 120.00, 2),
    ('Turkey Stew', '2.5 L', 85.00, 1),
    ('Turkey Stew', '5 L', 135.00, 2),
    ('Cooler of Jollof Rice', 'Full Cooler', 200.00, 1),
    ('Cooler of Jollof Rice', 'Half Cooler', 230.00, 2),
    ('Cooler of Fried Rice', 'Full Cooler', 300.00, 1),
    ('Cooler of Fried Rice', 'Half Cooler', 330.00, 2),
    ('Box of Peppered Turkey', 'Full Box', 200.00, 1),
    ('Box of Peppered Turkey', 'Half Box', 220.00, 2),
    ('Box of Peppered Chicken', 'Full Box', 150.00, 1),
    ('Box of Peppered Chicken', 'Half Box', 170.00, 2)
) AS v(item_name, label, price, display_order)
JOIN menu_items m ON m.name = v.item_name;
