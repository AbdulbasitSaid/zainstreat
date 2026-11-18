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
    ('Bulk Orders', NULL, 4),
    ('Snacks', 'Snacks, combo packs and small chops platters. Minimum orders apply.', 5);

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

-- Phase 36: the Snacks category's items, with per-item minimum orders
-- (same rows as migration 20261118160100_snacks_rentals_data).
-- Flat price, except Puff Puff (price options below).
WITH snacks AS (
    SELECT id FROM categories WHERE name = 'Snacks' AND deleted_at IS NULL ORDER BY id LIMIT 1
)
INSERT INTO menu_items (category_id, name, description, price, min_quantity, display_order)
SELECT s.id, v.name, v.description, v.price, v.min_quantity, v.display_order
FROM snacks s
CROSS JOIN (VALUES
    ('Meat Pie', 'Golden, flaky pastry filled with seasoned minced meat.', 4.00::NUMERIC(10, 2), 10, 1),
    ('Snack Pack', '1 samosa • 1 spring roll • 4 puff puff • 1 grilled chicken', 7.00, 10, 2),
    ('Puff Puff (small chops size)', 'Soft, sweet, deep-fried dough balls.', NULL, 1, 3),
    ('50 Puff Puff & 10 Meat Skewers', '50 puff puff and 10 meat skewers.', 75.00, 1, 4),
    ('Meat Skewer', 'Spiced, grilled meat on a stick.', 5.00, 10, 5),
    ('Snack Combo 1', '1 samosa • 1 spring roll • 4 puff puff • 1 grilled chicken • 1 meat pie', 12.00, 5, 6),
    ('Snack Combo 2', '2 samosas • 1 spring roll • 4 puff puff • 1 meat pie', 11.50, 5, 7),
    ('Snack Combo 3', '2 samosas • 2 spring rolls • 5 puff puff • 1 grilled chicken', 11.00, 5, 8),
    ('Snack Combo 4', '1 meat pie • 5 puff puff • 1 peppered chicken', 10.00, 5, 9),
    ('Snack Combo 5', '1 meat pie • 1 grilled chicken', 7.00, 5, 10),
    ('Silver Small Chops Platter', '10 samosas • 10 spring rolls • 30 puff puff • 10 grilled chicken', 70.00, 1, 11),
    ('Gold Small Chops Platter', '15 samosas • 15 spring rolls • 60 puff puff • 15 grilled chicken', 95.00, 1, 12),
    ('Diamond Small Chops Platter', '20 samosas • 20 spring rolls • 100 puff puff • 20 grilled chicken', 150.00, 1, 13)
) AS v(name, description, price, min_quantity, display_order)
WHERE NOT EXISTS (
    SELECT 1 FROM menu_items m
    WHERE m.category_id = s.id AND m.name = v.name AND m.deleted_at IS NULL
);

INSERT INTO menu_item_price_options (menu_item_id, label, price, display_order)
SELECT m.id, o.label, o.price, o.display_order
FROM menu_items m
JOIN categories c ON c.id = m.category_id AND c.name = 'Snacks' AND c.deleted_at IS NULL
CROSS JOIN (VALUES ('50 pieces', 30.00::NUMERIC(10, 2), 1), ('100 pieces', 45.00, 2))
    AS o(label, price, display_order)
WHERE m.name = 'Puff Puff (small chops size)'
  AND m.deleted_at IS NULL
  AND m.price IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM menu_item_price_options p
      WHERE p.menu_item_id = m.id AND p.deleted_at IS NULL
  );

-- Phase 36 rentals. Not truncated above: seed.sql reseeds the menu, and
-- the owner's rentals are separate, so this only adds missing ones.
INSERT INTO rentals (name, description, price, display_order)
SELECT v.name, v.description, v.price, v.display_order
FROM (VALUES
    ('Chafing Dish – 10 L', 'Keeps food warm on the buffet.', 15.00::NUMERIC(10, 2), 1),
    ('Chafing Dish – 5 L', 'Keeps food warm on the buffet.', 12.00, 2),
    ('Double Cocktail Dispenser', 'Two-tank drinks dispenser.', 25.00, 3),
    ('Large Cooking Pot', 'Holds up to 10 kg of rice.', 30.00, 4),
    ('Cooler', 'Keeps food or drinks hot or cold.', 10.00, 5),
    ('Electric Warmer Pot', 'Keeps dishes warm.', 10.00, 6),
    ('Gas Burner', 'Supports up to 50 kg.', 15.00, 7)
) AS v(name, description, price, display_order)
WHERE NOT EXISTS (SELECT 1 FROM rentals r WHERE r.name = v.name AND r.deleted_at IS NULL);
