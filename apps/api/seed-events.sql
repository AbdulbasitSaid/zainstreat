-- Phase 31 (requirement.md Decision 21): The Abula Experience.
-- Idempotent: upserts the event by slug, never truncates, never touches
-- bookings. Plates are only replaced while nobody has booked, so re-running
-- it on a live event can't orphan a guest's plate. Staging only for now.
--
-- PLACEHOLDER: the plate prices (€15), the area, the address and the cover
-- photo stand in until the client confirms them. Update them here (or in
-- admin) before this file is ever run against production.

BEGIN;

INSERT INTO events (slug, content, starts_at, ends_at, area, address, capacity, max_party_size,
                    meal_orders_close_at, payment_hold_hours, cover_image_url, published_at)
VALUES (
    'the-abula-experience-2026',
    $json${
      "en": {
        "title": "The Abula Experience",
        "summary": "A home-cooked plate of àmàlà, ewédú and gbẹ̀gìrì, and a shared table to meet people over the Christmas break.",
        "story": "Abula is the plate many of us miss most: soft àmàlà, silky ewédú and rich gbẹ̀gìrì, cooked slowly and served warm.\n\nOn the Sunday after Christmas we're opening our home for a late lunch at one shared table. Come for the food, stay for the conversation, and meet people who love a proper plate of Abula as much as we do. Not sure about swallow? There's smoky jollof rice with salad too.\n\nEntry is free. If you'd like to eat, choose your plate when you book: we cook to order, so every plate is reserved once it's paid.",
        "terms": "Meal orders close on 20 December. Cancel before then from your booking page and we'll refund your meals in full. After 20 December the ingredients are bought, so meals can't be refunded, but you're welcome to send someone in your place. Unpaid bookings are released 48 hours after booking.",
        "recap": ""
      },
      "nl": {
        "title": "De Abula-ervaring",
        "summary": "Een huisgemaakt bord àmàlà, ewédú en gbẹ̀gìrì, en een gedeelde tafel om in de kerstvakantie nieuwe mensen te ontmoeten.",
        "story": "Abula is het bord dat velen van ons het meest missen: zachte àmàlà, zijdezachte ewédú en romige gbẹ̀gìrì, langzaam gekookt en warm geserveerd.\n\nOp de zondag na Kerst openen we ons huis voor een late lunch aan één gedeelde tafel. Kom voor het eten, blijf voor de gesprekken, en ontmoet mensen die net zo van een goed bord Abula houden als wij. Twijfel je over swallow? Er is ook smoky jollofrijst met salade.\n\nDe toegang is gratis. Wil je mee-eten? Kies dan je bord bij het reserveren: we koken op bestelling, dus elk bord is gereserveerd zodra het betaald is.",
        "terms": "Maaltijden bestellen kan tot 20 december. Annuleer je daarvoor via je reserveringspagina, dan krijg je je maaltijden volledig terugbetaald. Na 20 december zijn de ingrediënten gekocht en kunnen we maaltijden niet meer terugbetalen, maar je mag iemand anders in je plaats sturen. Onbetaalde reserveringen vervallen 48 uur na het reserveren.",
        "recap": ""
      },
      "fr": {
        "title": "L'expérience Abula",
        "summary": "Une assiette faite maison d'àmàlà, ewédú et gbẹ̀gìrì, et une table partagée pour faire des rencontres pendant les fêtes de Noël.",
        "story": "L'Abula, c'est l'assiette qui manque le plus à beaucoup d'entre nous : un àmàlà moelleux, un ewédú soyeux et un gbẹ̀gìrì généreux, mijotés lentement et servis chauds.\n\nLe dimanche après Noël, nous ouvrons notre maison pour un déjeuner tardif autour d'une seule grande table. Venez pour la cuisine, restez pour les conversations, et rencontrez des gens qui aiment une bonne assiette d'Abula autant que nous. Pas sûr pour le « swallow » ? Il y a aussi du riz jollof fumé avec salade.\n\nL'entrée est gratuite. Si vous souhaitez manger, choisissez votre assiette en réservant : nous cuisinons sur commande, chaque assiette est donc réservée dès qu'elle est payée.",
        "terms": "Les commandes de repas ferment le 20 décembre. Annulez avant cette date depuis la page de votre réservation et vos repas vous seront intégralement remboursés. Après le 20 décembre, les ingrédients sont achetés : les repas ne sont plus remboursables, mais vous pouvez envoyer quelqu'un à votre place. Les réservations non payées sont libérées 48 heures après la réservation.",
        "recap": ""
      }
    }$json$::jsonb,
    '2026-12-27 14:00' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
    '2026-12-27 18:00' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
    'A home lunch in [city]',          -- PLACEHOLDER
    'Placeholder Street 1, [city]',    -- PLACEHOLDER
    16, 6,
    '2026-12-20 23:59' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
    48,
    '/images/events/abula-experience.jpg', -- PLACEHOLDER photo
    now()
)
ON CONFLICT (slug) DO UPDATE SET
    content = EXCLUDED.content,
    starts_at = EXCLUDED.starts_at,
    ends_at = EXCLUDED.ends_at,
    area = EXCLUDED.area,
    address = EXCLUDED.address,
    capacity = EXCLUDED.capacity,
    max_party_size = EXCLUDED.max_party_size,
    meal_orders_close_at = EXCLUDED.meal_orders_close_at,
    payment_hold_hours = EXCLUDED.payment_hold_hours,
    cover_image_url = EXCLUDED.cover_image_url,
    published_at = COALESCE(events.published_at, EXCLUDED.published_at),
    updated_at = now();

-- Plates: replaced only while the event has no bookings.
DELETE FROM event_plates
WHERE event_id = (SELECT id FROM events WHERE slug = 'the-abula-experience-2026')
  AND NOT EXISTS (
      SELECT 1 FROM event_bookings b
      JOIN events e ON e.id = b.event_id
      WHERE e.slug = 'the-abula-experience-2026'
  );

INSERT INTO event_plates (event_id, position, content, price)
SELECT e.id, v.position, v.content::jsonb, v.price
FROM events e
CROSS JOIN (VALUES
    (0, $json${
      "en": {"name": "Abula", "description": "Àmàlà with ewédú and gbẹ̀gìrì, served with assorted meat."},
      "nl": {"name": "Abula", "description": "Àmàlà met ewédú en gbẹ̀gìrì, geserveerd met gemengd vlees."},
      "fr": {"name": "Abula", "description": "Àmàlà avec ewédú et gbẹ̀gìrì, servi avec un assortiment de viandes."}
    }$json$, 15.00),   -- PLACEHOLDER price
    (1, $json${
      "en": {"name": "Smoky jollof rice with salad", "description": "Party-style jollof with a fresh salad."},
      "nl": {"name": "Smoky jollofrijst met salade", "description": "Jollofrijst zoals op een feest, met een frisse salade."},
      "fr": {"name": "Riz jollof fumé avec salade", "description": "Le jollof des grandes fêtes, avec une salade fraîche."}
    }$json$, 15.00)    -- PLACEHOLDER price
) AS v(position, content, price)
WHERE e.slug = 'the-abula-experience-2026'
  AND NOT EXISTS (SELECT 1 FROM event_plates p WHERE p.event_id = e.id);

COMMIT;
