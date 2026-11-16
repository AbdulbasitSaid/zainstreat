// Admin service worker (Phase 27). Registered on cms.<host> only, by
// components/admin-service-worker.tsx. Deliberately no fetch handler and
// no cache: the admin shows live order data (requirement.md Decision 5).
// Push handlers arrive in plan Group 5.

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));
