// Phase 25 Decision 23: viewport presets in CSS px, portrait. Frames render
// at the real width so media queries and Tailwind breakpoints fire exactly
// as on the device, then scale down to fit.

export interface DevicePreset {
  id: string;
  label: string;
  width: number;
  height: number;
  kind: "phone" | "tablet" | "desktop";
}

export const DEVICE_PRESETS: readonly DevicePreset[] = [
  { id: "mobile-s", label: "Mobile S", width: 320, height: 568, kind: "phone" },
  { id: "mobile-m", label: "Mobile M", width: 375, height: 667, kind: "phone" },
  { id: "mobile-l", label: "Mobile L", width: 414, height: 896, kind: "phone" },
  { id: "mobile-xl", label: "Mobile XL", width: 430, height: 932, kind: "phone" },
  { id: "tablet", label: "Tablet", width: 768, height: 1024, kind: "tablet" },
  { id: "tablet-l", label: "Tablet L", width: 1024, height: 1366, kind: "tablet" },
  { id: "laptop", label: "Laptop", width: 1280, height: 800, kind: "desktop" },
  { id: "desktop", label: "Desktop", width: 1440, height: 900, kind: "desktop" },
  { id: "desktop-xl", label: "Desktop XL", width: 1920, height: 1080, kind: "desktop" },
];

export const DEVICE_GROUPS = [
  { kind: "phone", label: "Phone" },
  { kind: "tablet", label: "Tablet" },
  { kind: "desktop", label: "Desktop" },
] as const;

export function devicePreset(id: string): DevicePreset {
  return DEVICE_PRESETS.find((preset) => preset.id === id) ?? DEVICE_PRESETS[1];
}

/** Rotation only applies to phones and tablets. */
export function deviceSize(preset: DevicePreset, rotated: boolean): { width: number; height: number } {
  return rotated && preset.kind !== "desktop"
    ? { width: preset.height, height: preset.width }
    : { width: preset.width, height: preset.height };
}

// The bezel or browser chrome added around the viewport, in screen px.
export const DESKTOP_CHROME_HEIGHT = 32;
export function bezelSize(kind: DevicePreset["kind"]): number {
  return kind === "phone" ? 10 : kind === "tablet" ? 8 : 1;
}
