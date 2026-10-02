-- Dev-only fixture data. Meant to run once against a fresh database, not
-- repeatedly (menu_items.name has no unique constraint, so a second run
-- would duplicate rows).
INSERT INTO categories (name, description) VALUES
    ('Rice Dishes', 'Classic rice-based meals'),
    ('Snacks', 'Light bites and finger foods'),
    ('Drinks', 'Beverages to go with your meal')
ON CONFLICT DO NOTHING;

INSERT INTO menu_items (category_id, name, description, price, image_url, is_available, is_featured)
SELECT c.id, v.name, v.description, v.price, v.image_url, true, v.is_featured
FROM (VALUES
    ('Rice Dishes', 'Jollof Rice', 'Smoky party-style jollof rice', 6000.00, 'https://placehold.co/600x400?text=Jollof+Rice', true),
    ('Rice Dishes', 'Fried Rice', 'Vegetable fried rice', 6000.00, 'https://placehold.co/600x400?text=Fried+Rice', false),
    ('Snacks', 'Chicken Suya', 'Grilled spiced chicken skewers', 3500.00, 'https://placehold.co/600x400?text=Chicken+Suya', true),
    ('Snacks', 'Puff Puff', 'Sweet fried dough balls (6 pcs)', 1500.00, 'https://placehold.co/600x400?text=Puff+Puff', false),
    ('Snacks', 'Meat Pie', 'Savory pastry with minced meat filling', 1000.00, 'https://placehold.co/600x400?text=Meat+Pie', false),
    ('Snacks', 'Chin Chin', 'Crunchy sweet fried snack (pack)', 1500.00, 'https://placehold.co/600x400?text=Chin+Chin', false)
) AS v(category_name, name, description, price, image_url, is_featured)
JOIN categories c ON c.name = v.category_name;
