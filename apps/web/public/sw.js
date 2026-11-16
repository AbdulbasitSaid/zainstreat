// Admin service worker (Phase 27). Registered on cms.<host> only, by
// components/admin-service-worker.tsx. Deliberately no fetch handler and
// no cache: the admin shows live order data (requirement.md Decision 5).

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

// Only same-origin admin paths: a push can't send the app elsewhere
// (Decision 13). The API never sends anything else; this is the backstop.
function adminPath(url) {
  return typeof url === "string" && url.startsWith("/") && !url.startsWith("//") ? url : "/";
}

function readPayload(event) {
  if (!event.data) return {};
  try {
    return event.data.json();
  } catch {
    return { body: event.data.text() };
  }
}

// Every push shows a notification: Safari revokes subscriptions that
// receive pushes without one.
self.addEventListener("push", (event) => {
  const data = readPayload(event);
  const tasks = [
    self.registration.showNotification(data.title || "Zain's Admin", {
      body: data.body || "",
      tag: data.tag,
      renotify: Boolean(data.tag),
      icon: "/admin-icons/icon-192.png",
      badge: "/admin-icons/icon-192.png",
      data: { url: adminPath(data.url) },
    }),
  ];
  // Orders needing action (Decision 11). Not every browser has the Badging API.
  if (typeof data.badge === "number" && "setAppBadge" in self.navigator) {
    tasks.push(
      (data.badge > 0 ? self.navigator.setAppBadge(data.badge) : self.navigator.clearAppBadge()).catch(() => {}),
    );
  }
  event.waitUntil(Promise.all(tasks));
});

// Focus an open admin window and take it to the page, or open one.
self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const target = new URL(adminPath(event.notification.data && event.notification.data.url), self.location.origin);
  event.waitUntil(
    (async () => {
      const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
      const client = windows.find((c) => new URL(c.url).origin === self.location.origin);
      if (client) {
        await client.focus();
        if ("navigate" in client) return client.navigate(target.href);
        return undefined;
      }
      return self.clients.openWindow(target.href);
    })(),
  );
});

// The browser rotated the subscription (risk 3): subscribe again and save
// it, so pushes keep arriving without a trip to Settings.
self.addEventListener("pushsubscriptionchange", (event) => {
  event.waitUntil(
    (async () => {
      const keyResponse = await fetch("/api/admin/push/public-key", { credentials: "same-origin" });
      if (!keyResponse.ok) return;
      const { public_key: publicKey } = await keyResponse.json();
      const subscription = await self.registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: publicKey,
      });
      await fetch("/api/admin/push/subscriptions", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(subscription.toJSON()),
      });
    })(),
  );
});
