"use client";

import { createContext, useContext, type ReactNode } from "react";

// True inside the admin's live preview (Phase 25 Decision 25): ordering
// and form submissions render disabled there.
const PreviewContext = createContext(false);

export function PreviewProvider({ preview, children }: { preview: boolean; children: ReactNode }) {
  return <PreviewContext.Provider value={preview}>{children}</PreviewContext.Provider>;
}

export function usePreview(): boolean {
  return useContext(PreviewContext);
}

export const PREVIEW_DISABLED_TITLE = "Disabled in preview";
