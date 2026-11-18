DELETE FROM rentals WHERE name IN (
    'Chafing Dish – 10 L', 'Chafing Dish – 5 L', 'Double Cocktail Dispenser',
    'Large Cooking Pot', 'Cooler', 'Electric Warmer Pot', 'Gas Burner'
);

-- Orders keep their snapshots: order_items.menu_item_id is ON DELETE SET NULL.
DELETE FROM menu_items
WHERE category_id IN (SELECT id FROM categories WHERE name = 'Snacks')
  AND name IN (
      'Meat Pie', 'Snack Pack', 'Puff Puff (small chops size)', '50 Puff Puff & 10 Meat Skewers',
      'Meat Skewer', 'Snack Combo 1', 'Snack Combo 2', 'Snack Combo 3', 'Snack Combo 4',
      'Snack Combo 5', 'Silver Small Chops Platter', 'Gold Small Chops Platter',
      'Diamond Small Chops Platter'
  );

DELETE FROM categories c
WHERE c.name = 'Snacks'
  AND NOT EXISTS (SELECT 1 FROM menu_items m WHERE m.category_id = c.id);
