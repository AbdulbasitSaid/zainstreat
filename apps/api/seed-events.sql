-- The Abula Experience (Phase 31 Decision 21, Phase 32 Decisions 9-11),
-- from the client's flyer. Never truncates and never touches bookings.
--
-- Run with three psql variables (the workflows pass them):
--   staging:    -v publish=true  -v upsert=true  -v address='Staging test address, Almere'
--   production: -v publish=false -v upsert=false -v address=''
-- publish: published at once, or a draft for the owner to publish.
-- upsert:  re-runs refresh the text, settings and plates; with false, an
--          existing event is left exactly as the owner edited it.
-- Without them it runs in the safe production mode.
--
-- PLACEHOLDER: both plates are €15 until the client sets the real prices
-- (in admin on production).

\if :{?publish}
\else
\set publish false
\endif
\if :{?upsert}
\else
\set upsert false
\endif
\if :{?address}
\else
\set address ''
\endif

BEGIN;

CREATE TEMP TABLE seed_event (id BIGINT, created BOOLEAN) ON COMMIT DROP;

WITH upserted AS (
    INSERT INTO events (slug, content, starts_at, ends_at, area, address, capacity, max_party_size,
                        meal_orders_close_at, payment_hold_hours, cover_image_url, published_at)
    VALUES (
        'the-abula-experience-2026',
        $json${
          "en": {
            "title": "The Abula Experience",
            "summary": "A taste of home, a table of connection: a home-cooked plate of àmàlà, ewédú and gbẹ̀gìrì, and a shared table to meet people over the Christmas break.",
            "story": "Abula is the plate many of us miss most: soft àmàlà, silky ewédú and rich gbẹ̀gìrì with a flavourful stew, cooked slowly and served warm.\n\nOn the Sunday after Christmas we're opening our home in Almere for a late lunch at one shared table. Come for the food, stay for the conversation, and meet people who love a proper plate of Abula as much as we do.\n\nThere are two plates to choose from. The Classic Abula comes with assorted meat or fish. Not sure about swallow? The Smoky Jollof Experience is rich, smoky party jollof with fried plantain, moin moin or a fresh salad, and chicken or turkey.\n\nEntry is free and seats are limited. If you'd like to eat, choose your plate when you book: we cook to order, so every plate is reserved once it's paid.",
            "terms": "Meal orders close on 20 December. Cancel before then from your booking page and we'll refund your meals in full. After 20 December the ingredients are bought, so meals can't be refunded, but you're welcome to send someone in your place. Unpaid bookings are released 48 hours after booking.",
            "recap": ""
          },
          "nl": {
            "title": "De Abula-ervaring",
            "summary": "Een smaak van thuis, een tafel vol verbinding: een huisgemaakt bord àmàlà, ewédú en gbẹ̀gìrì, en een gedeelde tafel om in de kerstvakantie nieuwe mensen te ontmoeten.",
            "story": "Abula is het bord dat velen van ons het meest missen: zachte àmàlà, zijdezachte ewédú en romige gbẹ̀gìrì met een smaakvolle stoofsaus, langzaam gekookt en warm geserveerd.\n\nOp de zondag na Kerst openen we ons huis in Almere voor een late lunch aan één gedeelde tafel. Kom voor het eten, blijf voor de gesprekken, en ontmoet mensen die net zo van een goed bord Abula houden als wij.\n\nJe kiest uit twee borden. De klassieke Abula serveren we met gemengd vlees of vis. Twijfel je over swallow? De smoky jollof-ervaring is rijke, smoky feest-jollof met gebakken bakbanaan, moin moin of een verse salade, en kip of kalkoen.\n\nDe toegang is gratis en er zijn maar een paar plekken. Wil je mee-eten? Kies dan je bord bij het reserveren: we koken op bestelling, dus elk bord is gereserveerd zodra het betaald is.",
            "terms": "Maaltijden bestellen kan tot 20 december. Annuleer je daarvoor via je reserveringspagina, dan krijg je je maaltijden volledig terugbetaald. Na 20 december zijn de ingrediënten gekocht en kunnen we maaltijden niet meer terugbetalen, maar je mag iemand anders in je plaats sturen. Onbetaalde reserveringen vervallen 48 uur na het reserveren.",
            "recap": ""
          },
          "fr": {
            "title": "L'expérience Abula",
            "summary": "Un goût de chez soi, une table pour se retrouver : une assiette faite maison d'àmàlà, ewédú et gbẹ̀gìrì, et une table partagée pour faire des rencontres pendant les fêtes de Noël.",
            "story": "L'Abula, c'est l'assiette qui manque le plus à beaucoup d'entre nous : un àmàlà moelleux, un ewédú soyeux et un gbẹ̀gìrì généreux avec une sauce savoureuse, mijotés lentement et servis chauds.\n\nLe dimanche après Noël, nous ouvrons notre maison à Almere pour un déjeuner tardif autour d'une seule grande table. Venez pour la cuisine, restez pour les conversations, et rencontrez des gens qui aiment une bonne assiette d'Abula autant que nous.\n\nDeux assiettes au choix. L'Abula classique se sert avec un assortiment de viandes ou du poisson. Pas sûr pour le « swallow » ? L'expérience jollof fumé, c'est un riz jollof des grandes fêtes, avec bananes plantain frites, moin moin ou salade fraîche, et poulet ou dinde.\n\nL'entrée est gratuite et les places sont limitées. Si vous souhaitez manger, choisissez votre assiette en réservant : nous cuisinons sur commande, chaque assiette est donc réservée dès qu'elle est payée.",
            "terms": "Les commandes de repas ferment le 20 décembre. Annulez avant cette date depuis la page de votre réservation et vos repas vous seront intégralement remboursés. Après le 20 décembre, les ingrédients sont achetés : les repas ne sont plus remboursables, mais vous pouvez envoyer quelqu'un à votre place. Les réservations non payées sont libérées 48 heures après la réservation.",
            "recap": ""
          }
        }$json$::jsonb,
        '2026-12-27 14:00' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
        '2026-12-27 18:00' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
        'Almere',
        :'address',
        16, 6,
        '2026-12-20 23:59' ::timestamp AT TIME ZONE 'Europe/Amsterdam',
        48,
        '/images/events/abula-experience.jpg',
        CASE WHEN :'publish' = 'true' THEN now() END
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
        updated_at = now()
    WHERE :'upsert' = 'true'
    -- xmax is 0 on a freshly inserted row.
    RETURNING id, (xmax = 0) AS created
)
INSERT INTO seed_event SELECT id, created FROM upserted;

-- The flyer's two plates, with their option groups (Phase 32 Decision 9).
CREATE TEMP TABLE seed_plates (position INTEGER, content JSONB, options JSONB, price NUMERIC(10, 2)) ON COMMIT DROP;
INSERT INTO seed_plates VALUES
    (0, $json${
      "en": {"name": "The Classic Abula", "description": "Traditional àmàlà with rich gbẹ̀gìrì, fresh ewédú and a flavourful stew."},
      "nl": {"name": "De klassieke Abula", "description": "Traditionele àmàlà met romige gbẹ̀gìrì, verse ewédú en een smaakvolle stoofsaus."},
      "fr": {"name": "L'Abula classique", "description": "Àmàlà traditionnel avec un gbẹ̀gìrì généreux, de l'ewédú frais et une sauce savoureuse."}
    }$json$, $json$[
      {"key": "protein", "label": {"en": "Protein", "nl": "Vlees of vis", "fr": "Protéine"}, "choices": [
        {"key": "assorted-meat", "label": {"en": "Assorted meat", "nl": "Gemengd vlees", "fr": "Assortiment de viandes"}},
        {"key": "fish", "label": {"en": "Fish", "nl": "Vis", "fr": "Poisson"}}]}
    ]$json$, 15.00),   -- PLACEHOLDER price
    (1, $json${
      "en": {"name": "The Smoky Jollof Experience", "description": "Rich, smoky Nigerian party jollof rice with your choice of side and protein."},
      "nl": {"name": "De smoky jollof-ervaring", "description": "Rijke, smoky Nigeriaanse feest-jollofrijst met een bijgerecht en vlees naar keuze."},
      "fr": {"name": "L'expérience jollof fumé", "description": "Un riz jollof nigérian fumé, comme dans les grandes fêtes, avec l'accompagnement et la protéine de votre choix."}
    }$json$, $json$[
      {"key": "side", "label": {"en": "Side", "nl": "Bijgerecht", "fr": "Accompagnement"}, "choices": [
        {"key": "fried-plantain", "label": {"en": "Fried plantain", "nl": "Gebakken bakbanaan", "fr": "Bananes plantain frites"}},
        {"key": "moin-moin", "label": {"en": "Moin moin", "nl": "Moin moin", "fr": "Moin moin"}},
        {"key": "fresh-salad", "label": {"en": "Fresh salad", "nl": "Verse salade", "fr": "Salade fraîche"}}]},
      {"key": "protein", "label": {"en": "Protein", "nl": "Vlees", "fr": "Protéine"}, "choices": [
        {"key": "chicken", "label": {"en": "Chicken", "nl": "Kip", "fr": "Poulet"}},
        {"key": "turkey", "label": {"en": "Turkey", "nl": "Kalkoen", "fr": "Dinde"}}]}
    ]$json$, 15.00);   -- PLACEHOLDER price

-- Staging refresh with no bookings yet: replace the plates outright.
DELETE FROM event_plates p
USING seed_event s
WHERE p.event_id = s.id AND NOT s.created
  AND NOT EXISTS (SELECT 1 FROM event_bookings b WHERE b.event_id = s.id);

-- Staging refresh after test bookings: update the plates in place, keeping
-- their ids and prices so the bookings stay linked (Phase 32 open risk 2).
UPDATE event_plates p
SET content = v.content, options = v.options
FROM seed_event s, seed_plates v
WHERE p.event_id = s.id AND p.position = v.position
  AND EXISTS (SELECT 1 FROM event_bookings b WHERE b.event_id = s.id);

-- A new event, or a staging event whose plates were just cleared.
INSERT INTO event_plates (event_id, position, content, options, price)
SELECT s.id, v.position, v.content, v.options, v.price
FROM seed_event s CROSS JOIN seed_plates v
WHERE NOT EXISTS (SELECT 1 FROM event_plates p WHERE p.event_id = s.id);

COMMIT;
