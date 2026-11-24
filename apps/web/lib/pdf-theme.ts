// Phase 38 Decision 28: react-pdf can't read CSS variables, so the brand
// colours are copied from app/globals.css's @theme. Keep the two in sync.
export const PDF_COLORS = {
  primary: "#72123C",
  primaryDark: "#4E0C2A",
  accent: "#C66C84",
  accentLight: "#F0AEC0",
  backgroundSoft: "#FAE8EA",
  cream: "#FBF4E9",
  text: "#2B1114",
  textMuted: "#55343A",
  white: "#FFFFFF",
} as const;
