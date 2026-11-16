// Browser-side web push helpers for the admin Settings page (Phase 27,
// plan 5.3). Client-only: every function touches `navigator`/`window`.

export type PushSupport = "supported" | "needs-install" | "unsupported";

export type PushErrorReason = "denied" | "not-configured" | "failed";

export class PushError extends Error {
  constructor(readonly reason: PushErrorReason) {
    super(reason);
  }
}

export type PushDevice = {
  id: number;
  endpoint: string;
  label: string;
  created_at: string;
  last_success_at: string | null;
  last_failure_at: string | null;
};

interface NavigatorStandalone {
  standalone?: boolean;
}

function isIos(): boolean {
  return (
    /iPhone|iPad|iPod/.test(navigator.userAgent) ||
    (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)
  );
}

function isStandalone(): boolean {
  const nav: Navigator & NavigatorStandalone = navigator;
  return window.matchMedia("(display-mode: standalone)").matches || nav.standalone === true;
}

export function pushSupport(): PushSupport {
  if ("serviceWorker" in navigator && "PushManager" in window && "Notification" in window) {
    return "supported";
  }
  // iOS only exposes PushManager to the Home Screen app.
  return isIos() && !isStandalone() ? "needs-install" : "unsupported";
}

async function registration(): Promise<ServiceWorkerRegistration> {
  // Registered by AdminServiceWorker; `ready` waits for it to activate.
  return navigator.serviceWorker.ready;
}

export async function currentSubscription(): Promise<PushSubscription | null> {
  return (await registration()).pushManager.getSubscription();
}

export async function fetchPublicKey(): Promise<string> {
  const response = await fetch("/api/admin/push/public-key", { cache: "no-store" });
  if (response.status === 404) throw new PushError("not-configured");
  if (!response.ok) throw new PushError("failed");
  const { public_key: publicKey } = (await response.json()) as { public_key: string };
  return publicKey;
}

function base64UrlToBytes(value: string): Uint8Array<ArrayBuffer> {
  const base64 = (value + "=".repeat((4 - (value.length % 4)) % 4)).replace(/-/g, "+").replace(/_/g, "/");
  const raw = atob(base64);
  const bytes = new Uint8Array(new ArrayBuffer(raw.length));
  for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
  return bytes;
}

/** Must run from a click: iOS only shows the permission prompt for one. */
export async function subscribe(): Promise<void> {
  const permission = await Notification.requestPermission();
  if (permission !== "granted") throw new PushError("denied");

  const publicKey = await fetchPublicKey();
  const pushManager = (await registration()).pushManager;
  let subscription = await pushManager.getSubscription();
  // A subscription made with an older key pair can't receive anything.
  if (subscription && subscription.options.applicationServerKey) {
    const existingKey = new Uint8Array(subscription.options.applicationServerKey);
    const wanted = base64UrlToBytes(publicKey);
    const same = existingKey.length === wanted.length && existingKey.every((byte, i) => byte === wanted[i]);
    if (!same) {
      await subscription.unsubscribe();
      subscription = null;
    }
  }
  subscription ??= await pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: base64UrlToBytes(publicKey),
  });

  const response = await fetch("/api/admin/push/subscriptions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(subscription.toJSON()),
  });
  if (!response.ok) throw new PushError("failed");
}

export async function listDevices(): Promise<PushDevice[]> {
  const response = await fetch("/api/admin/push/subscriptions", { cache: "no-store" });
  if (!response.ok) throw new PushError("failed");
  return (await response.json()) as PushDevice[];
}

export async function removeDevice(id: number): Promise<void> {
  const response = await fetch(`/api/admin/push/subscriptions/${id}`, { method: "DELETE" });
  // Already gone (removed elsewhere, or a 410 cleaned it up) is fine.
  if (!response.ok && response.status !== 404) throw new PushError("failed");
}

/** Turn off on this device: forget it on the server, then in the browser. */
export async function unsubscribe(): Promise<void> {
  const subscription = await currentSubscription();
  if (!subscription) return;
  const device = (await listDevices()).find((d) => d.endpoint === subscription.endpoint);
  if (device) await removeDevice(device.id);
  await subscription.unsubscribe();
}

export async function sendTest(): Promise<void> {
  const response = await fetch("/api/admin/push/test", { method: "POST" });
  if (response.status === 404) throw new PushError("not-configured");
  if (!response.ok) throw new PushError("failed");
}
