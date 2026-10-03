-- Dev-only fixture data. Meant to run once against a fresh database, not
-- repeatedly (menu_items.name has no unique constraint, so a second run
-- would duplicate rows).
INSERT INTO categories (name, description) VALUES
    ('Rice Dishes', 'Classic rice-based meals'),
    ('Snacks', 'Light bites and finger foods'),
    ('Drinks', 'Beverages to go with your meal'),
    ('Soups', 'Hearty Nigerian soups, best paired with a swallow')
ON CONFLICT DO NOTHING;

-- is_available and is_featured are tracked as two independent columns
-- below (not derived from one shared flag) so the fixtures can exercise
-- both the menu page's "unavailable item" state and the homepage's
-- "featured item" state at once (Meat Pie is the one unavailable item).
INSERT INTO menu_items (category_id, name, description, price, image_url, is_available, is_featured)
SELECT c.id, v.name, v.description, v.price, v.image_url, v.is_available, v.is_featured
FROM (VALUES
    ('Rice Dishes', 'Jollof Rice', 'Smoky party-style jollof rice', 12.00, 'https://placehold.co/600x400?text=Jollof+Rice', true, true),
    ('Rice Dishes', 'Fried Rice', 'Vegetable fried rice', 12.00, 'https://placehold.co/600x400?text=Fried+Rice', true, false),
    ('Snacks', 'Chicken Suya', 'Grilled spiced chicken skewers', 8.50, 'https://placehold.co/600x400?text=Chicken+Suya', true, true),
    ('Snacks', 'Puff Puff', 'Sweet fried dough balls (6 pcs)', 4.00, 'https://placehold.co/600x400?text=Puff+Puff', true, false),
    ('Snacks', 'Meat Pie', 'Savory pastry with minced meat filling', 3.00, 'https://placehold.co/600x400?text=Meat+Pie', false, false),
    ('Snacks', 'Chin Chin', 'Crunchy sweet fried snack (pack)', 4.00, 'https://placehold.co/600x400?text=Chin+Chin', true, false),
    ('Snacks', 'Small Chops', 'A delicious selection of samosas, spring rolls, puff-puff, and grilled chicken — perfect for parties and celebrations', 15.00, '/images/menu/small-chops.jpeg', true, true),
    ('Soups', 'Efo Soup', 'Rich and flavorful Nigerian spinach stew with beef, smoked catfish, shaki, crayfish, ponmo, and palm oil', 14.00, '/images/menu/efo-soup.jpeg', true, true),
    ('Soups', 'Egusi Soup', 'Rich and flavorful Nigerian soup made with ground melon seeds, palm oil, crayfish, beef, shaki, and ponmo, finished with fresh spinach', 14.00, '/images/menu/egusi-soup.jpeg', true, true)
) AS v(category_name, name, description, price, image_url, is_available, is_featured)
JOIN categories c ON c.name = v.category_name;

-- Real photo + copy for Chin Chin, superseding the placehold.co placeholder
-- above (menu_items has no unique constraint, so this is an UPDATE, not a
-- second INSERT, to avoid duplicating the row).
UPDATE menu_items
SET description = 'Classic crunchy and milky Nigerian snack made from flour, sugar, milk, and delicious flavoring, deep-fried until golden and crisp',
    image_url = '/images/menu/chin-chin.jpeg'
WHERE name = 'Chin Chin';
