-- Phase 36: the client's snack menu and rentals list (requirement.md
-- Decisions 6, 7, 12). Insert-if-missing only: it never updates or
-- deletes, so the owner's later edits in admin win.
--
-- Only a database that already has a menu (staging, production) gets the
-- rows: the test databases start empty and must stay that way, and a fresh
-- dev database gets the same rows from seed.sql.

INSERT INTO categories (name, description, display_order)
SELECT 'Snacks', 'Snacks, combo packs and small chops platters. Minimum orders apply.',
       COALESCE(MAX(display_order), 0) + 1
FROM categories
WHERE deleted_at IS NULL
HAVING COUNT(*) > 0
   AND NOT EXISTS (SELECT 1 FROM categories WHERE name = 'Snacks' AND deleted_at IS NULL);

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
WHERE EXISTS (SELECT 1 FROM categories WHERE name = 'Snacks' AND deleted_at IS NULL)
  AND NOT EXISTS (SELECT 1 FROM rentals r WHERE r.name = v.name AND r.deleted_at IS NULL);
