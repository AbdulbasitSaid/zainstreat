"use client";

import { useCallback, useEffect, useState } from "react";
import { AdminConfirmDialog } from "@/components/admin-confirm-dialog";
import { Button } from "@/components/button";
import { formatDateTime } from "@/lib/format";
import {
  PushError,
  currentSubscription,
  fetchPublicKey,
  listDevices,
  pushSupport,
  removeDevice,
  sendTest,
  subscribe,
  unsubscribe,
  type PushDevice,
} from "@/lib/push-client";

type DeviceState = "loading" | "unsupported" | "needs-install" | "not-configured" | "blocked" | "off" | "on";
type Action = "on" | "off" | "test" | "remove";

const STATE_TEXT: Record<DeviceState, string> = {
  loading: "Checking this device…",
  unsupported: "This browser doesn't support notifications.",
  "needs-install": "Add this app to your Home Screen first, then open it from there to turn on notifications.",
  "not-configured": "Notifications aren't set up on this server yet.",
  blocked:
    "Notifications are blocked for this app. On iPhone, open Settings → Notifications → Zain's Admin. On Android or a computer, allow notifications in the browser's site settings. Then come back here.",
  off: "Notifications are off for this device.",
  on: "Notifications are on for this device.",
};

const ERROR_TEXT: Record<PushError["reason"], string> = {
  denied: "Permission wasn't given, so notifications stay off.",
  "not-configured": "Notifications aren't set up on this server yet.",
  failed: "Something went wrong. Please try again.",
};

function errorText(error: unknown): string {
  return error instanceof PushError ? ERROR_TEXT[error.reason] : ERROR_TEXT.failed;
}

async function detectState(): Promise<{ state: DeviceState; endpoint: string | null }> {
  const support = pushSupport();
  if (support !== "supported") return { state: support, endpoint: null };
  try {
    await fetchPublicKey();
  } catch (error) {
    if (error instanceof PushError && error.reason === "not-configured") {
      return { state: "not-configured", endpoint: null };
    }
  }
  if (Notification.permission === "denied") return { state: "blocked", endpoint: null };
  const subscription = await currentSubscription();
  if (subscription && Notification.permission === "granted") {
    return { state: "on", endpoint: subscription.endpoint };
  }
  return { state: "off", endpoint: null };
}

type Loaded = { state: DeviceState; endpoint: string | null; devices: PushDevice[] | null };

async function load(): Promise<Loaded> {
  const detected = await detectState();
  return { ...detected, devices: await listDevices().catch(() => null) };
}

// Notifications section of Settings (Phase 27 Decision 9): this device's
// state, on/off/test, and every device that gets notifications.
export function AdminNotifications() {
  const [state, setState] = useState<DeviceState>("loading");
  const [endpoint, setEndpoint] = useState<string | null>(null);
  const [devices, setDevices] = useState<PushDevice[] | null>(null);
  const [busy, setBusy] = useState<Action | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [removing, setRemoving] = useState<PushDevice | null>(null);

  const apply = useCallback((loaded: Loaded) => {
    setState(loaded.state);
    setEndpoint(loaded.endpoint);
    setDevices(loaded.devices);
  }, []);

  const refresh = useCallback(async () => apply(await load()), [apply]);

  useEffect(() => {
    let cancelled = false;
    void load().then((loaded) => {
      if (!cancelled) apply(loaded);
    });
    return () => {
      cancelled = true;
    };
  }, [apply]);

  async function run(action: Action, task: () => Promise<void>, success: string) {
    setBusy(action);
    setMessage(null);
    try {
      await task();
      setMessage(success);
    } catch (error) {
      setMessage(errorText(error));
    } finally {
      await refresh();
      setBusy(null);
    }
  }

  const canTest = state === "on" || (devices !== null && devices.length > 0);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 rounded-2xl border border-text/10 p-5">
        <p className="m-0 font-semibold" aria-live="polite">
          {STATE_TEXT[state]}
        </p>
        <div className="flex flex-wrap gap-3">
          {(state === "off" || state === "blocked") && (
            <Button
              type="button"
              disabled={busy !== null || state === "blocked"}
              onClick={() =>
                run("on", subscribe, "Notifications are on. Send a test to check this device.")
              }
            >
              {busy === "on" ? "Turning on…" : "Turn on notifications"}
            </Button>
          )}
          {state === "on" && (
            <Button
              type="button"
              variant="secondary"
              disabled={busy !== null}
              onClick={() => run("off", unsubscribe, "Notifications are off for this device.")}
            >
              {busy === "off" ? "Turning off…" : "Turn off"}
            </Button>
          )}
          {canTest && state !== "not-configured" && (
            <Button
              type="button"
              variant="secondary"
              disabled={busy !== null}
              onClick={() =>
                run("test", sendTest, "Test sent. It should arrive on every device below in a few seconds.")
              }
            >
              {busy === "test" ? "Sending…" : "Send test notification"}
            </Button>
          )}
        </div>
        {message && (
          <p className="m-0 text-sm text-text-muted" role="status">
            {message}
          </p>
        )}
      </div>

      <div className="flex flex-col gap-3">
        <h3 className="m-0 text-base">Devices</h3>
        {devices === null ? (
          <p className="m-0 text-sm text-text-muted">Couldn&apos;t load devices.</p>
        ) : devices.length === 0 ? (
          <p className="m-0 text-sm text-text-muted">No devices get notifications yet.</p>
        ) : (
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {devices.map((device) => {
              const failing =
                device.last_failure_at !== null &&
                (device.last_success_at === null || device.last_failure_at > device.last_success_at);
              return (
                <li
                  key={device.id}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-text/10 px-4 py-3"
                >
                  <div className="flex flex-col gap-0.5">
                    <span className="font-semibold">
                      {device.label}
                      {device.endpoint === endpoint && (
                        <span className="ml-2 rounded-full bg-primary px-2 py-0.5 text-[0.7rem] text-white">
                          This device
                        </span>
                      )}
                    </span>
                    <span className="text-xs text-text-muted">
                      Added {formatDateTime(device.created_at)} · Last delivered{" "}
                      {device.last_success_at ? formatDateTime(device.last_success_at) : "never"}
                    </span>
                    {failing && <span className="text-xs font-semibold text-primary">Delivery failing</span>}
                  </div>
                  <button
                    type="button"
                    className="text-sm font-semibold text-text-muted underline"
                    disabled={busy !== null}
                    onClick={() => setRemoving(device)}
                  >
                    Remove
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <AdminConfirmDialog
        open={removing !== null}
        title="Remove this device?"
        confirmLabel="Remove"
        busy={busy === "remove"}
        onCancel={() => setRemoving(null)}
        onConfirm={() => {
          const device = removing;
          if (!device) return;
          void run(
            "remove",
            async () => {
              // Removing this device also unsubscribes the browser.
              if (device.endpoint === endpoint) await unsubscribe();
              else await removeDevice(device.id);
            },
            `${device.label} won't get notifications any more.`,
          ).then(() => setRemoving(null));
        }}
      >
        {removing?.label} will stop getting notifications. It can be turned on again from Settings
        on that device.
      </AdminConfirmDialog>
    </div>
  );
}
