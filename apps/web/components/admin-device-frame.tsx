"use client";

import { forwardRef } from "react";
import { DESKTOP_CHROME_HEIGHT, bezelSize, type DevicePreset } from "@/lib/devices";

/**
 * One simulated device (Phase 25 plan 10.2): an iframe at the device's real
 * CSS size, scaled down with `transform` so the page lays out exactly as on
 * the device. Before `mounted`, only the empty bezel is drawn, so "All
 * devices" doesn't load nine pages at once.
 */
export const AdminDeviceFrame = forwardRef<
  HTMLIFrameElement,
  {
    preset: DevicePreset;
    width: number;
    height: number;
    scale: number;
    src: string;
    path: string;
    mounted: boolean;
    failed: boolean;
    onLoad: () => void;
    onOpenInNewTab: () => void;
  }
>(function AdminDeviceFrame({ preset, width, height, scale, src, path, mounted, failed, onLoad, onOpenInNewTab }, ref) {
  const bezel = bezelSize(preset.kind);
  const chrome = preset.kind === "desktop" ? DESKTOP_CHROME_HEIGHT : 0;
  const viewportWidth = Math.round(width * scale);
  const viewportHeight = Math.round(height * scale);
  const shape =
    preset.kind === "phone"
      ? "rounded-[2.5rem] border-text"
      : preset.kind === "tablet"
        ? "rounded-xl border-text"
        : "rounded-lg border-text/20";

  return (
    <figure className="m-0 flex shrink-0 flex-col items-center gap-2">
      <div
        className={`overflow-hidden bg-background ${shape}`}
        style={{ borderWidth: bezel, width: viewportWidth + bezel * 2, height: viewportHeight + chrome + bezel * 2 }}
      >
        {preset.kind === "desktop" && (
          <div
            className="flex items-center gap-1.5 border-b border-text/10 bg-background-soft px-3"
            style={{ height: chrome }}
          >
            <span className="h-2.5 w-2.5 rounded-full bg-primary/50" />
            <span className="h-2.5 w-2.5 rounded-full bg-accent" />
            <span className="h-2.5 w-2.5 rounded-full bg-whatsapp/60" />
            <span className="ml-3 truncate rounded-full bg-background px-3 py-0.5 text-[0.7rem] text-text-muted">
              {path || "/"}
            </span>
          </div>
        )}
        <div className="relative overflow-hidden" style={{ width: viewportWidth, height: viewportHeight }}>
          {failed ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 p-4 text-center text-sm">
              <p className="m-0 font-semibold">Preview couldn&apos;t start here</p>
              <button type="button" onClick={onOpenInNewTab} className="font-semibold text-primary underline">
                Open in a new tab
              </button>
            </div>
          ) : (
            mounted && (
              <iframe
                ref={ref}
                src={src}
                title={`${preset.label} preview`}
                width={width}
                height={height}
                onLoad={onLoad}
                className="absolute top-0 left-0 border-0 bg-background"
                style={{ width, height, transform: `scale(${scale})`, transformOrigin: "top left" }}
              />
            )
          )}
        </div>
      </div>
      <figcaption className="text-xs text-text-muted">
        {preset.label} · {width} × {height} · {Math.round(scale * 100)}%
      </figcaption>
    </figure>
  );
});
