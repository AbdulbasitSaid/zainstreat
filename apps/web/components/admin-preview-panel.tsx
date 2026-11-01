"use client";

import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "@/components/button";
import { AdminDeviceSimulator, type DeviceSimulatorHandle } from "@/components/admin-device-simulator";

const MIN_WIDTH = 360;
const MAX_WIDTH = 900;
const DEFAULT_WIDTH = 520;
const DESKTOP_QUERY = "(min-width: 1024px)";

const RefreshPreviewContext = createContext<() => void>(() => {});

/** For editors: refreshes the docked preview after a draft save. */
export function useRefreshPreview(): () => void {
  return useContext(RefreshPreviewContext);
}

/**
 * An editor with the device simulator docked beside it (Phase 25 plan
 * 10.4): resizable from 360 to 900px, toggled with "Preview", hidden by
 * default below `lg` and full screen on a phone.
 */
export function AdminPreviewPanel({ previewPath, children }: { previewPath: string; children: ReactNode }) {
  const simulatorRef = useRef<DeviceSimulatorHandle>(null);
  const [open, setOpen] = useState(false);
  const [width, setWidth] = useState(DEFAULT_WIDTH);
  const dragRef = useRef<{ startX: number; startWidth: number } | null>(null);

  useEffect(() => {
    // matchMedia only exists after hydration.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setOpen(window.matchMedia(DESKTOP_QUERY).matches);
  }, []);

  const refresh = useCallback(() => simulatorRef.current?.refresh(), []);

  function startDrag(event: React.PointerEvent<HTMLDivElement>) {
    dragRef.current = { startX: event.clientX, startWidth: width };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function drag(event: React.PointerEvent<HTMLDivElement>) {
    if (!dragRef.current) return;
    const next = dragRef.current.startWidth + (dragRef.current.startX - event.clientX);
    setWidth(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, next)));
  }

  return (
    <RefreshPreviewContext.Provider value={refresh}>
      <div className="flex flex-col gap-6 lg:flex-row lg:items-start">
        <div className="min-w-0 flex-1">
          <div className="mb-4 flex justify-end">
            <Button type="button" variant="secondary" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
              {open ? "Hide preview" : "Preview"}
            </Button>
          </div>
          {children}
        </div>

        {open && (
          <aside
            aria-label="Live preview"
            className="fixed inset-0 z-50 flex flex-col bg-background p-3 lg:sticky lg:inset-auto lg:top-4 lg:z-auto lg:h-[calc(100dvh-2rem)] lg:w-[var(--panel-width)] lg:shrink-0 lg:flex-row lg:bg-transparent lg:p-0"
            style={{ "--panel-width": `${width}px` } as React.CSSProperties}
          >
            <div
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize preview"
              onPointerDown={startDrag}
              onPointerMove={drag}
              onPointerUp={() => (dragRef.current = null)}
              className="hidden w-2 shrink-0 cursor-col-resize rounded-full bg-text/10 hover:bg-accent lg:block"
            />
            <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-2 lg:pl-3">
              <div className="flex justify-end lg:hidden">
                <Button type="button" variant="secondary" onClick={() => setOpen(false)}>
                  Close preview
                </Button>
              </div>
              <AdminDeviceSimulator ref={simulatorRef} initialPath={previewPath} />
            </div>
          </aside>
        )}
      </div>
    </RefreshPreviewContext.Provider>
  );
}
