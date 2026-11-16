"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import { Button } from "@/components/button";

// Not in TypeScript's DOM lib: Chromium's install prompt event and iOS
// Safari's `navigator.standalone`. Only the members used here.
interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed" }>;
}

interface NavigatorStandalone {
  standalone?: boolean;
}

type Platform = "server" | "installed" | "ios" | "other";

const STANDALONE_QUERY = "(display-mode: standalone)";

function subscribe(onChange: () => void) {
  const query = window.matchMedia(STANDALONE_QUERY);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

function getPlatform(): Platform {
  const nav: Navigator & NavigatorStandalone = navigator;
  if (window.matchMedia(STANDALONE_QUERY).matches || nav.standalone === true) return "installed";
  // iPadOS reports itself as a Mac; touch points give it away.
  const isIos =
    /iPhone|iPad|iPod/.test(navigator.userAgent) ||
    (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
  return isIos ? "ios" : "other";
}

// Install hint on Settings (Phase 27 Decision 4): steps on iOS, where a
// page can't prompt; a button where Chromium offers a prompt; nothing once
// installed.
export function AdminInstallHint() {
  const platform = useSyncExternalStore<Platform>(subscribe, getPlatform, () => "server");
  const [installPrompt, setInstallPrompt] = useState<BeforeInstallPromptEvent | null>(null);

  useEffect(() => {
    function onPrompt(event: Event) {
      event.preventDefault();
      setInstallPrompt(event as BeforeInstallPromptEvent);
    }
    function onInstalled() {
      setInstallPrompt(null);
    }
    window.addEventListener("beforeinstallprompt", onPrompt);
    window.addEventListener("appinstalled", onInstalled);
    return () => {
      window.removeEventListener("beforeinstallprompt", onPrompt);
      window.removeEventListener("appinstalled", onInstalled);
    };
  }, []);

  async function install() {
    if (!installPrompt) return;
    await installPrompt.prompt();
    await installPrompt.userChoice;
    setInstallPrompt(null);
  }

  if (platform === "server" || platform === "installed") return null;

  if (platform === "ios") {
    return (
      <div className="flex flex-col gap-2 rounded-2xl border border-text/10 bg-background-soft/40 p-5">
        <p className="m-0 font-semibold">Add Zain&apos;s Admin to your Home Screen</p>
        <p className="m-0 text-sm text-text-muted">
          On iPhone, notifications only work from the Home Screen app. In Safari, tap the Share
          button, then <strong>Add to Home Screen</strong>, and open the admin from the new icon.
        </p>
      </div>
    );
  }

  if (!installPrompt) return null;

  return (
    <div className="flex flex-col items-start gap-3 rounded-2xl border border-text/10 bg-background-soft/40 p-5">
      <p className="m-0 text-sm text-text-muted">
        Install the admin as an app for quick access and notifications.
      </p>
      <Button type="button" onClick={install}>
        Install app
      </Button>
    </div>
  );
}
