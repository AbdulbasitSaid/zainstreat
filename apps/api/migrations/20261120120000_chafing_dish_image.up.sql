-- The Chafing Dish rentals show the same photo as the homepage's Catering
-- Rentals tile. Only fills a missing image, so an owner's upload in admin wins.
UPDATE rentals
SET image_url = '/images/rentals/chafing-dish.jpg'
WHERE name IN ('Chafing Dish – 10 L', 'Chafing Dish – 5 L')
  AND image_url IS NULL
  AND deleted_at IS NULL;
