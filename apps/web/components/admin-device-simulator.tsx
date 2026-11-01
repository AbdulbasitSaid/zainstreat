"use client";

import { useCallback, useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import { AdminDeviceFrame } from "@/components/admin-device-frame";
import { routing, type AppLocale } from "@/i18n/routing";
import {
  DESKTOP_CHROME_HEIGHT,
  DEVICE_GROUPS,
  DEVICE_PRESETS,
  bezelSize,
  devicePreset,
  deviceSize,
  type DevicePreset,
} from "@/lib/devices";
import { PREVIEW_PAGES, normalizePreviewPath, previewEnableUrl } from "@/lib/preview-paths";
import type { PreviewMessage } from "@/components/preview-bridge";

export interface DeviceSimulatorHandle {
  /** Re-renders the visible frames with the latest draft. */
  refresh: () => void;
}

type Zoom = "fit" | 0.5 | 0.75 | 1;
const ZOOMS: readonly Zoom[] = ["fit", 0.5, 0.75, 1];
const ALL_DEVICES_HEIGHT = 560;
const REFRESH_STAGGER_MS = 150;
// Risk 8: a frame that hasn't reported its path this long after loading
// couldn't start draft mode (e.g. a logged-out session or blocked cookies).
const STARTUP_TIMEOUT_MS = 5000;
const STORAGE_KEY = "zt-admin-simulator";

interface StoredSettings {
  presetId?: string;
  zoom?: Zoom;
  locale?: AppLocale;
  path?: string;
}

function readSettings(): StoredSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as StoredSettings) : {};
  } catch {
    return {};
  }
}

function writeSettings(settings: StoredSettings) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Private mode or blocked storage: the settings just aren't remembered.
  }
}

interface FrameState {
  mounted: boolean;
  visible: boolean;
  stale: boolean;
  ready: boolean;
  failed: boolean;
}

const INITIAL_FRAME: FrameState = { mounted: false, visible: false, stale: false, ready: false, failed: false };

/**
 * The admin's device simulator (Phase 25 Decisions 23–24): the real public
 * pages, with draft data, in one device or all nine at once. No library —
 * iframes at real CSS widths, scaled with `transform`.
 */
export function AdminDeviceSimulator({
  initialPath,
  ref,
}: {
  /** The page the edited content affects; remembered when omitted. */
  initialPath?: string;
  ref?: Ref<DeviceSimulatorHandle>;
}) {
  const [presetId, setPresetId] = useState(DEVICE_PRESETS[1].id);
  const [allDevices, setAllDevices] = useState(false);
  const [rotated, setRotated] = useState(false);
  const [zoom, setZoom] = useState<Zoom>("fit");
  const [locale, setLocale] = useState<AppLocale>(routing.defaultLocale);
  const [requestedPath, setRequestedPath] = useState(normalizePreviewPath(initialPath ?? "") ?? "");
  const [currentPath, setCurrentPath] = useState(requestedPath);
  const [loaded, setLoaded] = useState(false);
  const [frames, setFrames] = useState<Record<string, FrameState>>({});
  const [available, setAvailable] = useState({ width: 0, height: 0 });
  // Bumped to reload every frame (and retry failed ones) at the same src.
  const [generation, setGeneration] = useState(0);

  const stageRef = useRef<HTMLDivElement>(null);
  const iframes = useRef(new Map<string, HTMLIFrameElement>());
  const slots = useRef(new Map<string, HTMLDivElement>());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const framesRef = useRef(frames);

  useEffect(() => {
    framesRef.current = frames;
  });

  // Restore the remembered settings once, on the client.
  useEffect(() => {
    const stored = readSettings();
    /* eslint-disable react-hooks/set-state-in-effect -- localStorage only exists after hydration */
    if (stored.presetId) setPresetId(devicePreset(stored.presetId).id);
    if (stored.zoom && ZOOMS.includes(stored.zoom)) setZoom(stored.zoom);
    if (stored.locale && (routing.locales as readonly string[]).includes(stored.locale)) setLocale(stored.locale);
    if (initialPath === undefined && stored.path !== undefined) {
      const path = normalizePreviewPath(stored.path);
      if (path !== null) {
        setRequestedPath(path);
        setCurrentPath(path);
      }
    }
    setLoaded(true);
    /* eslint-enable react-hooks/set-state-in-effect */
  }, [initialPath]);

  useEffect(() => {
    if (!loaded) return;
    writeSettings({ presetId, zoom, locale, ...(initialPath === undefined ? { path: requestedPath } : {}) });
  }, [loaded, presetId, zoom, locale, requestedPath, initialPath]);

  // The available stage size, for "Fit".
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const observer = new ResizeObserver(([entry]) => {
      setAvailable({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(stage);
    return () => observer.disconnect();
  }, []);

  const visiblePresets: readonly DevicePreset[] = allDevices ? DEVICE_PRESETS : [devicePreset(presetId)];
  const src = previewEnableUrl(locale, requestedPath);

  const updateFrame = useCallback((id: string, patch: Partial<FrameState>) => {
    setFrames((current) => ({ ...current, [id]: { ...INITIAL_FRAME, ...current[id], ...patch } }));
  }, []);

  const postRefresh = useCallback(
    (id: string) => {
      const message: PreviewMessage = { type: "zt:refresh" };
      iframes.current.get(id)?.contentWindow?.postMessage(message, window.location.origin);
      updateFrame(id, { stale: false });
    },
    [updateFrame],
  );

  // A new page, locale or generation reloads every mounted frame.
  useEffect(() => {
    /* eslint-disable-next-line react-hooks/set-state-in-effect -- resets per-frame startup state for the new src */
    setFrames((current) =>
      Object.fromEntries(
        Object.entries(current).map(([id, frame]) => [id, { ...frame, ready: false, failed: false, stale: false }]),
      ),
    );
  }, [src, generation]);

  // Mount a frame once it's within 200px of the viewport (Risk 7); a
  // single-device view mounts straight away. Mounted frames stay mounted.
  useEffect(() => {
    if (!allDevices) {
      /* eslint-disable-next-line react-hooks/set-state-in-effect -- the single frame is always visible */
      updateFrame(presetId, { mounted: true, visible: true });
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const id = (entry.target as HTMLElement).dataset.presetId;
          if (!id) continue;
          if (entry.isIntersecting) {
            const wasStale = framesRef.current[id]?.stale;
            updateFrame(id, { mounted: true, visible: true });
            if (wasStale) postRefresh(id);
          } else {
            updateFrame(id, { visible: false });
          }
        }
      },
      { rootMargin: "200px" },
    );
    for (const slot of slots.current.values()) observer.observe(slot);
    return () => observer.disconnect();
  }, [allDevices, presetId, updateFrame, postRefresh]);

  // Frames report their path once PreviewBridge is running.
  useEffect(() => {
    function onMessage(event: MessageEvent<PreviewMessage>) {
      if (event.origin !== window.location.origin || event.data?.type !== "zt:path") return;
      for (const [id, iframe] of iframes.current) {
        if (iframe.contentWindow !== event.source) continue;
        clearTimeout(timers.current.get(id));
        updateFrame(id, { ready: true, failed: false });
        const path = event.data.path.replace(/^\/[a-z]{2}(?=\/|$|\?)/, "");
        setCurrentPath(path);
      }
    }
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [updateFrame]);

  useEffect(() => {
    const pending = timers.current;
    return () => {
      for (const timer of pending.values()) clearTimeout(timer);
    };
  }, []);

  function handleLoad(id: string) {
    clearTimeout(timers.current.get(id));
    if (framesRef.current[id]?.ready) return;
    timers.current.set(
      id,
      setTimeout(() => {
        if (!framesRef.current[id]?.ready) updateFrame(id, { failed: true });
      }, STARTUP_TIMEOUT_MS),
    );
  }

  const openInNewTab = useCallback(() => {
    window.open(src, "_blank", "noopener");
  }, [src]);

  useImperativeHandle(
    ref,
    () => ({
      refresh() {
        // Visible frames only, staggered; hidden ones refresh when they
        // scroll back into view (Risk 7).
        let delay = 0;
        for (const [id, frame] of Object.entries(framesRef.current)) {
          if (!frame.mounted || frame.failed) continue;
          if (frame.visible) {
            setTimeout(() => postRefresh(id), delay);
            delay += REFRESH_STAGGER_MS;
          } else {
            updateFrame(id, { stale: true });
          }
        }
      },
    }),
    [postRefresh, updateFrame],
  );

  function scaleFor(preset: DevicePreset, width: number, height: number): number {
    const bezel = bezelSize(preset.kind) * 2;
    const chrome = preset.kind === "desktop" ? DESKTOP_CHROME_HEIGHT : 0;
    if (allDevices) return Math.min(1, (ALL_DEVICES_HEIGHT - chrome - bezel) / height);
    if (zoom !== "fit") return zoom;
    if (available.width === 0) return 0.25;
    return Math.max(
      0.1,
      Math.min(1, (available.width - bezel - 8) / width, (available.height - chrome - bezel - 32) / height),
    );
  }

  const singlePreset = devicePreset(presetId);
  const canRotate = !allDevices && singlePreset.kind !== "desktop";
  const pickerPath = currentPath.split("?")[0];

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-col gap-2 text-xs">
        <div className="flex flex-wrap items-center gap-1.5">
          {DEVICE_GROUPS.map((group) => (
            <div key={group.kind} className="flex items-center gap-1 rounded-full bg-background-soft p-1" role="group" aria-label={group.label}>
              <span className="px-1.5 font-semibold text-text-muted">{group.label}</span>
              {DEVICE_PRESETS.filter((preset) => preset.kind === group.kind).map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  aria-pressed={!allDevices && presetId === preset.id}
                  onClick={() => {
                    setAllDevices(false);
                    setPresetId(preset.id);
                  }}
                  title={`${preset.width} × ${preset.height}`}
                  className={`rounded-full px-2.5 py-1 font-semibold ${
                    !allDevices && presetId === preset.id ? "bg-primary text-white" : "hover:bg-background"
                  }`}
                >
                  {preset.label.replace(/^(Mobile|Tablet|Desktop) ?/, "") || preset.label}
                </button>
              ))}
            </div>
          ))}
          <button
            type="button"
            aria-pressed={allDevices}
            onClick={() => setAllDevices((value) => !value)}
            className={`rounded-full px-3 py-1.5 font-semibold ${allDevices ? "bg-primary text-white" : "bg-background-soft"}`}
          >
            All devices
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <label className="flex items-center gap-1">
            <span className="sr-only">Page</span>
            <select
              value={PREVIEW_PAGES.some((page) => page.path === pickerPath) ? pickerPath : ""}
              onChange={(event) => {
                setRequestedPath(event.target.value);
                setCurrentPath(event.target.value);
              }}
              className="rounded-full border border-text/15 bg-background px-2 py-1"
            >
              {PREVIEW_PAGES.map((page) => (
                <option key={page.path} value={page.path}>
                  {page.label}
                </option>
              ))}
            </select>
          </label>
          <div className="flex items-center rounded-full bg-background-soft p-1" role="group" aria-label="Language">
            {routing.locales.map((code) => (
              <button
                key={code}
                type="button"
                aria-pressed={locale === code}
                onClick={() => setLocale(code)}
                className={`rounded-full px-2 py-0.5 font-semibold uppercase ${locale === code ? "bg-primary text-white" : ""}`}
              >
                {code}
              </button>
            ))}
          </div>
          {!allDevices && (
            <div className="flex items-center rounded-full bg-background-soft p-1" role="group" aria-label="Zoom">
              {ZOOMS.map((value) => (
                <button
                  key={String(value)}
                  type="button"
                  aria-pressed={zoom === value}
                  onClick={() => setZoom(value)}
                  className={`rounded-full px-2 py-0.5 font-semibold ${zoom === value ? "bg-primary text-white" : ""}`}
                >
                  {value === "fit" ? "Fit" : `${value * 100}%`}
                </button>
              ))}
            </div>
          )}
          {canRotate && (
            <button
              type="button"
              aria-pressed={rotated}
              onClick={() => setRotated((value) => !value)}
              className="rounded-full bg-background-soft px-3 py-1 font-semibold"
            >
              Rotate
            </button>
          )}
          <button
            type="button"
            onClick={() => setGeneration((value) => value + 1)}
            className="rounded-full bg-background-soft px-3 py-1 font-semibold"
          >
            Reload
          </button>
          <button type="button" onClick={openInNewTab} className="rounded-full bg-background-soft px-3 py-1 font-semibold">
            Open in new tab
          </button>
        </div>
      </div>

      <div
        ref={stageRef}
        className={`min-h-0 flex-1 rounded-card bg-background-soft/60 p-2 ${
          allDevices ? "overflow-x-auto overflow-y-hidden" : zoom === "fit" ? "overflow-hidden" : "overflow-auto"
        }`}
      >
        <div className={`flex gap-6 ${allDevices ? "w-max items-end" : "justify-center"}`}>
          {visiblePresets.map((preset) => {
            const { width, height } = deviceSize(preset, !allDevices && rotated);
            const frame = frames[preset.id] ?? INITIAL_FRAME;
            return (
              <div
                key={preset.id}
                data-preset-id={preset.id}
                ref={(node) => {
                  if (node) slots.current.set(preset.id, node);
                  else slots.current.delete(preset.id);
                }}
              >
                <AdminDeviceFrame
                  key={`${src}#${generation}`}
                  ref={(node) => {
                    if (node) iframes.current.set(preset.id, node);
                    else iframes.current.delete(preset.id);
                  }}
                  preset={preset}
                  width={width}
                  height={height}
                  scale={scaleFor(preset, width, height)}
                  src={src}
                  path={`/${locale}${currentPath}`}
                  mounted={loaded && frame.mounted}
                  failed={frame.failed}
                  onLoad={() => handleLoad(preset.id)}
                  onOpenInNewTab={openInNewTab}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
